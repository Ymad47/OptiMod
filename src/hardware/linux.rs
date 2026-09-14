use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::CString,
    fmt::Write as _,
    fs, io,
    mem::MaybeUninit,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use super::{
    AcceleratorSnapshot, CpuSnapshot, HostSnapshot, INSPECTION_SCHEMA_VERSION, InspectionError,
    InspectionReport, MemorySnapshot, StorageSnapshot,
};

const PROC_CPUINFO: &str = "/proc/cpuinfo";
const PROC_MEMINFO: &str = "/proc/meminfo";
const PROC_MOUNTINFO: &str = "/proc/self/mountinfo";
const PROC_CGROUP: &str = "/proc/self/cgroup";
const CGROUP_ROOT: &str = "/sys/fs/cgroup";
const DRM_ROOT: &str = "/sys/class/drm";
const NUMA_ROOT: &str = "/sys/devices/system/node";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CgroupMemory {
    limit_bytes: u64,
    used_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MountInfo {
    mount_point: PathBuf,
    major_minor: String,
    filesystem: String,
    device: String,
}

pub(super) fn inspect(path: &Path) -> Result<InspectionReport, InspectionError> {
    let mut warnings = Vec::new();

    let operating_system = match read_optional(Path::new("/etc/os-release")) {
        Ok(Some(value)) => parse_os_release(&value).unwrap_or_else(|| {
            warnings.push("/etc/os-release has no NAME or PRETTY_NAME".to_owned());
            "Linux".to_owned()
        }),
        Ok(None) => {
            warnings.push("/etc/os-release is unavailable".to_owned());
            "Linux".to_owned()
        }
        Err(source) => {
            warnings.push(format!("could not read /etc/os-release: {source}"));
            "Linux".to_owned()
        }
    };

    let cgroup_directory = current_cgroup_directory();
    let cgroup_quota = read_cgroup_cpu_quota(cgroup_directory.as_deref(), &mut warnings);
    let cgroup_memory = read_cgroup_memory(cgroup_directory.as_deref(), &mut warnings);

    let cpuinfo = fs::read_to_string(PROC_CPUINFO)
        .map_err(|source| InspectionError::io("read", PROC_CPUINFO, source))?;
    let available_parallelism = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or_else(|source| {
            warnings.push(format!(
                "could not determine available parallelism: {source}"
            ));
            1
        });
    let cpu = parse_cpuinfo(
        &cpuinfo,
        available_parallelism,
        cgroup_quota,
        discover_numa_nodes(),
    )
    .map_err(|detail| InspectionError::invalid("/proc/cpuinfo", detail))?;

    let meminfo = fs::read_to_string(PROC_MEMINFO)
        .map_err(|source| InspectionError::io("read", PROC_MEMINFO, source))?;
    let memory = parse_meminfo(&meminfo, cgroup_memory)
        .map_err(|detail| InspectionError::invalid("/proc/meminfo", detail))?;

    let accelerators = discover_accelerators(&mut warnings);
    let storage = inspect_storage(path, &mut warnings)?;

    Ok(InspectionReport {
        schema_version: INSPECTION_SCHEMA_VERSION,
        host: HostSnapshot {
            operating_system,
            architecture: std::env::consts::ARCH.to_owned(),
            cpu,
            memory,
            accelerators,
            storage,
        },
        warnings,
    })
}

pub(super) fn render_human(report: &InspectionReport) -> String {
    let host = &report.host;
    let mut output = String::new();

    writeln!(
        output,
        "OptiMod host inspection (schema {})",
        report.schema_version
    )
    .expect("writing to String cannot fail");
    writeln!(output, "Operating system: {}", host.operating_system)
        .expect("writing to String cannot fail");
    writeln!(output, "Architecture: {}", host.architecture).expect("writing to String cannot fail");

    writeln!(output, "\nCPU").expect("writing to String cannot fail");
    writeln!(output, "  Model: {}", host.cpu.model).expect("writing to String cannot fail");
    writeln!(output, "  Logical CPUs: {}", host.cpu.logical_cpus)
        .expect("writing to String cannot fail");
    writeln!(
        output,
        "  Available parallelism: {}",
        host.cpu.available_parallelism
    )
    .expect("writing to String cannot fail");
    if let Some(physical_cores) = host.cpu.physical_cores {
        writeln!(output, "  Physical cores: {physical_cores}")
            .expect("writing to String cannot fail");
    }
    if let Some(quota) = host.cpu.cgroup_quota_cpus {
        writeln!(output, "  Cgroup CPU quota: {quota:.2} CPUs")
            .expect("writing to String cannot fail");
    }
    if let Some(nodes) = host.cpu.numa_nodes {
        writeln!(output, "  NUMA nodes: {nodes}").expect("writing to String cannot fail");
    }
    writeln!(
        output,
        "  Inference features: {}",
        if host.cpu.features.is_empty() {
            "none detected".to_owned()
        } else {
            host.cpu.features.join(", ")
        }
    )
    .expect("writing to String cannot fail");

    writeln!(output, "\nMemory").expect("writing to String cannot fail");
    writeln!(
        output,
        "  Host total: {}",
        format_bytes(host.memory.host_total_bytes)
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "  Host available: {}",
        format_bytes(host.memory.host_available_bytes)
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "  Effective total: {}",
        format_bytes(host.memory.effective_total_bytes)
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "  Effective available: {}",
        format_bytes(host.memory.effective_available_bytes)
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "  Swap: {} total, {} available",
        format_bytes(host.memory.swap_total_bytes),
        format_bytes(host.memory.swap_available_bytes)
    )
    .expect("writing to String cannot fail");
    if let Some(limit) = host.memory.cgroup_limit_bytes {
        writeln!(output, "  Cgroup limit: {}", format_bytes(limit))
            .expect("writing to String cannot fail");
    }

    writeln!(output, "\nAccelerators").expect("writing to String cannot fail");
    if host.accelerators.is_empty() {
        writeln!(output, "  None detected through Linux DRM")
            .expect("writing to String cannot fail");
    } else {
        for accelerator in &host.accelerators {
            write!(output, "  {} [{}]", accelerator.name, accelerator.backend)
                .expect("writing to String cannot fail");
            if let Some(total) = accelerator.memory_total_bytes {
                write!(output, ": {} total", format_bytes(total))
                    .expect("writing to String cannot fail");
            }
            writeln!(output).expect("writing to String cannot fail");
        }
    }

    writeln!(output, "\nStorage").expect("writing to String cannot fail");
    writeln!(output, "  Path: {}", host.storage.path.display())
        .expect("writing to String cannot fail");
    if let Some(filesystem) = &host.storage.filesystem {
        writeln!(output, "  Filesystem: {filesystem}").expect("writing to String cannot fail");
    }
    if let Some(device) = &host.storage.device {
        writeln!(output, "  Device: {device}").expect("writing to String cannot fail");
    }
    writeln!(
        output,
        "  Capacity: {} total, {} available",
        format_bytes(host.storage.total_bytes),
        format_bytes(host.storage.available_bytes)
    )
    .expect("writing to String cannot fail");
    if let Some(rotational) = host.storage.rotational {
        writeln!(
            output,
            "  Rotational: {}",
            if rotational { "yes" } else { "no" }
        )
        .expect("writing to String cannot fail");
    }

    if !report.warnings.is_empty() {
        writeln!(output, "\nWarnings").expect("writing to String cannot fail");
        for warning in &report.warnings {
            writeln!(output, "  - {warning}").expect("writing to String cannot fail");
        }
    }

    output
}

fn parse_cpuinfo(
    input: &str,
    available_parallelism: usize,
    cgroup_quota_cpus: Option<f64>,
    numa_nodes: Option<usize>,
) -> Result<CpuSnapshot, String> {
    if available_parallelism == 0 {
        return Err("available parallelism cannot be zero".to_owned());
    }

    let mut model_name = None;
    let mut hardware_name = None;
    let mut cpu_model = None;
    let mut descriptive_processor = None;
    let mut logical_cpus = 0_usize;
    let mut core_ids = BTreeSet::new();
    let mut features = BTreeSet::new();

    for block in input.split("\n\n") {
        let fields = parse_key_value_lines(block);
        if fields.is_empty() {
            continue;
        }

        if fields.contains_key("processor") {
            logical_cpus += 1;
        }

        model_name = model_name.or_else(|| non_empty_field(&fields, "model name"));
        hardware_name = hardware_name.or_else(|| non_empty_field(&fields, "hardware"));
        cpu_model = cpu_model.or_else(|| non_empty_field(&fields, "cpu model"));
        descriptive_processor = descriptive_processor.or_else(|| {
            non_empty_field(&fields, "processor")
                .filter(|value| value.chars().any(char::is_alphabetic))
        });

        let package = fields
            .get("physical id")
            .or_else(|| fields.get("package id"));
        if let Some(core) = fields.get("core id") {
            core_ids.insert((
                package.cloned().unwrap_or_else(|| "0".to_owned()),
                core.clone(),
            ));
        }

        for value in [fields.get("flags"), fields.get("features")]
            .into_iter()
            .flatten()
        {
            for feature in value
                .split_whitespace()
                .filter(|value| is_inference_feature(value))
            {
                features.insert(feature.to_ascii_lowercase());
            }
        }
    }

    if logical_cpus == 0 {
        logical_cpus = available_parallelism;
    }

    let model = model_name
        .or(hardware_name)
        .or(cpu_model)
        .or(descriptive_processor)
        .ok_or_else(|| "CPU model is missing".to_owned())?;

    Ok(CpuSnapshot {
        model,
        physical_cores: (!core_ids.is_empty()).then_some(core_ids.len()),
        logical_cpus,
        available_parallelism,
        cgroup_quota_cpus,
        features: features.into_iter().collect(),
        numa_nodes,
    })
}

fn non_empty_field(fields: &BTreeMap<String, String>, key: &str) -> Option<String> {
    fields.get(key).filter(|value| !value.is_empty()).cloned()
}

fn parse_meminfo(input: &str, cgroup: Option<CgroupMemory>) -> Result<MemorySnapshot, String> {
    let values = parse_key_value_lines(input);
    let host_total_bytes = parse_kib_field(&values, "memtotal")?;
    let host_available_bytes = values
        .get("memavailable")
        .or_else(|| values.get("memfree"))
        .ok_or_else(|| "MemAvailable and MemFree are missing".to_owned())
        .and_then(|value| parse_kib(value, "MemAvailable"))?;
    let swap_total_bytes = values
        .get("swaptotal")
        .map_or(Ok(0), |value| parse_kib(value, "SwapTotal"))?;
    let swap_available_bytes = values
        .get("swapfree")
        .map_or(Ok(0), |value| parse_kib(value, "SwapFree"))?;

    let (effective_total_bytes, effective_available_bytes, cgroup_limit_bytes, cgroup_used_bytes) =
        if let Some(cgroup) = cgroup {
            (
                host_total_bytes.min(cgroup.limit_bytes),
                host_available_bytes.min(cgroup.limit_bytes.saturating_sub(cgroup.used_bytes)),
                Some(cgroup.limit_bytes),
                Some(cgroup.used_bytes),
            )
        } else {
            (host_total_bytes, host_available_bytes, None, None)
        };

    Ok(MemorySnapshot {
        host_total_bytes,
        host_available_bytes,
        effective_total_bytes,
        effective_available_bytes,
        swap_total_bytes,
        swap_available_bytes,
        cgroup_limit_bytes,
        cgroup_used_bytes,
    })
}

fn parse_kib_field(values: &BTreeMap<String, String>, key: &str) -> Result<u64, String> {
    values
        .get(key)
        .ok_or_else(|| format!("{key} is missing"))
        .and_then(|value| parse_kib(value, key))
}

fn parse_kib(value: &str, field: &str) -> Result<u64, String> {
    let mut parts = value.split_whitespace();
    let amount = parts
        .next()
        .ok_or_else(|| format!("{field} is empty"))?
        .parse::<u64>()
        .map_err(|source| format!("{field} is not an integer: {source}"))?;
    if let Some(unit) = parts.next() {
        if !unit.eq_ignore_ascii_case("kb") {
            return Err(format!("{field} has unsupported unit {unit}"));
        }
    }
    amount
        .checked_mul(1024)
        .ok_or_else(|| format!("{field} overflows bytes"))
}

fn parse_cpu_quota(input: &str) -> Result<Option<f64>, String> {
    let mut fields = input.split_whitespace();
    let quota = fields
        .next()
        .ok_or_else(|| "cpu.max quota is missing".to_owned())?;
    let period = fields
        .next()
        .ok_or_else(|| "cpu.max period is missing".to_owned())?
        .parse::<u64>()
        .map_err(|source| format!("cpu.max period is invalid: {source}"))?;
    if fields.next().is_some() {
        return Err("cpu.max has extra fields".to_owned());
    }
    if period == 0 {
        return Err("cpu.max period cannot be zero".to_owned());
    }
    if quota == "max" {
        return Ok(None);
    }
    let quota = quota
        .parse::<u64>()
        .map_err(|source| format!("cpu.max quota is invalid: {source}"))?;
    if quota == 0 {
        return Err("cpu.max quota cannot be zero".to_owned());
    }
    Ok(Some(quota as f64 / period as f64))
}

fn parse_os_release(input: &str) -> Option<String> {
    let values = input
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim(), unquote_os_release(value.trim())))
        })
        .collect::<BTreeMap<_, _>>();

    values
        .get("PRETTY_NAME")
        .or_else(|| values.get("NAME"))
        .filter(|value| !value.is_empty())
        .cloned()
}

fn unquote_os_release(value: &str) -> String {
    let value = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value);
    value.replace("\\\"", "\"").replace("\\\\", "\\")
}

fn parse_mountinfo(path: &Path, input: &str) -> Option<MountInfo> {
    input
        .lines()
        .filter_map(|line| {
            let (left, right) = line.split_once(" - ")?;
            let left = left.split_whitespace().collect::<Vec<_>>();
            let right = right.split_whitespace().collect::<Vec<_>>();
            if left.len() < 5 || right.len() < 2 {
                return None;
            }
            let mount_point = PathBuf::from(decode_mount_field(left[4]));
            if !path.starts_with(&mount_point) {
                return None;
            }
            Some(MountInfo {
                mount_point,
                major_minor: left[2].to_owned(),
                filesystem: decode_mount_field(right[0]),
                device: decode_mount_field(right[1]),
            })
        })
        .max_by_key(|mount| mount.mount_point.components().count())
}

fn decode_mount_field(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\'
            && index + 3 < bytes.len()
            && bytes[index + 1..=index + 3].iter().all(u8::is_ascii_digit)
        {
            if let Ok(octal) = std::str::from_utf8(&bytes[index + 1..=index + 3]) {
                if let Ok(byte) = u8::from_str_radix(octal, 8) {
                    decoded.push(byte);
                    index += 4;
                    continue;
                }
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn parse_drm_accelerator(
    vendor: &str,
    device: &str,
    uevent: &str,
    memory_total: Option<&str>,
    memory_used: Option<&str>,
) -> Result<Option<AcceleratorSnapshot>, String> {
    let vendor_id = vendor.trim().to_ascii_lowercase();
    let device_id = device.trim().to_ascii_lowercase();
    if vendor_id.is_empty() || device_id.is_empty() {
        return Err("DRM vendor or device ID is empty".to_owned());
    }

    let backend = match vendor_id.as_str() {
        "0x10de" => "nvidia",
        "0x1002" => "amd",
        "0x8086" => "intel",
        _ => return Ok(None),
    }
    .to_owned();
    let driver = uevent.lines().find_map(|line| line.strip_prefix("DRIVER="));
    let name = match driver {
        Some(driver) => format!("{backend} GPU ({driver})"),
        None => format!("{backend} GPU ({device_id})"),
    };
    let memory_total_bytes = memory_total
        .map(str::trim)
        .map(str::parse::<u64>)
        .transpose()
        .map_err(|source| format!("VRAM total is invalid: {source}"))?;
    let memory_used_bytes = memory_used
        .map(str::trim)
        .map(str::parse::<u64>)
        .transpose()
        .map_err(|source| format!("VRAM usage is invalid: {source}"))?;
    let memory_available_bytes = match (memory_total_bytes, memory_used_bytes) {
        (Some(total), Some(used)) => Some(total.saturating_sub(used)),
        _ => None,
    };

    Ok(Some(AcceleratorSnapshot {
        backend,
        name,
        vendor_id: Some(vendor_id),
        device_id: Some(device_id),
        memory_total_bytes,
        memory_available_bytes,
    }))
}

fn inspect_storage(
    path: &Path,
    warnings: &mut Vec<String>,
) -> Result<StorageSnapshot, InspectionError> {
    let canonical_path =
        fs::canonicalize(path).map_err(|source| InspectionError::io("resolve", path, source))?;
    let c_path = CString::new(canonical_path.as_os_str().as_bytes()).map_err(|_| {
        InspectionError::invalid("storage path", "path contains an interior NUL byte")
    })?;
    let mut stats = MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: c_path is NUL-terminated and stats points to valid writable memory.
    let result = unsafe { libc::statvfs(c_path.as_ptr(), stats.as_mut_ptr()) };
    if result != 0 {
        return Err(InspectionError::io(
            "inspect filesystem containing",
            canonical_path,
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: statvfs returned success and initialized the output structure.
    let stats = unsafe { stats.assume_init() };
    let fragment_size = into_u64(if stats.f_frsize == 0 {
        stats.f_bsize
    } else {
        stats.f_frsize
    });
    let total_bytes = into_u64(stats.f_blocks).saturating_mul(fragment_size);
    let available_bytes = into_u64(stats.f_bavail).saturating_mul(fragment_size);

    let mount = match read_optional(Path::new(PROC_MOUNTINFO)) {
        Ok(Some(value)) => parse_mountinfo(&canonical_path, &value),
        Ok(None) => None,
        Err(source) => {
            warnings.push(format!("could not read {PROC_MOUNTINFO}: {source}"));
            None
        }
    };
    if mount.is_none() {
        warnings.push(format!(
            "could not map {} to a mounted filesystem",
            canonical_path.display()
        ));
    }
    let rotational = mount
        .as_ref()
        .and_then(|mount| discover_rotational(&mount.major_minor));

    Ok(StorageSnapshot {
        path: canonical_path,
        filesystem: mount.as_ref().map(|mount| mount.filesystem.clone()),
        device: mount.as_ref().map(|mount| mount.device.clone()),
        total_bytes,
        available_bytes,
        rotational,
    })
}

fn into_u64<T>(value: T) -> u64
where
    T: TryInto<u64>,
{
    value.try_into().unwrap_or(u64::MAX)
}

fn discover_rotational(major_minor: &str) -> Option<bool> {
    let mut path = fs::canonicalize(Path::new("/sys/dev/block").join(major_minor)).ok()?;
    loop {
        let candidate = path.join("queue/rotational");
        if let Ok(value) = fs::read_to_string(candidate) {
            return match value.trim() {
                "0" => Some(false),
                "1" => Some(true),
                _ => None,
            };
        }
        if path == Path::new("/sys") || !path.pop() {
            return None;
        }
    }
}

fn discover_numa_nodes() -> Option<usize> {
    let count = fs::read_dir(NUMA_ROOT)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.strip_prefix("node").is_some_and(all_ascii_digits))
        })
        .count();
    (count > 0).then_some(count)
}

fn discover_accelerators(warnings: &mut Vec<String>) -> Vec<AcceleratorSnapshot> {
    let entries = match fs::read_dir(DRM_ROOT) {
        Ok(entries) => entries,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(source) => {
            warnings.push(format!("could not inspect {DRM_ROOT}: {source}"));
            return Vec::new();
        }
    };

    let mut accelerators = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(index) = name.strip_prefix("card") else {
            continue;
        };
        if !all_ascii_digits(index) {
            continue;
        }

        let device_path = entry.path().join("device");
        let Some(vendor) = read_optional(&device_path.join("vendor")).ok().flatten() else {
            continue;
        };
        let Some(device) = read_optional(&device_path.join("device")).ok().flatten() else {
            continue;
        };
        let uevent = read_optional(&device_path.join("uevent"))
            .ok()
            .flatten()
            .unwrap_or_default();
        let memory_total = read_optional(&device_path.join("mem_info_vram_total"))
            .ok()
            .flatten();
        let memory_used = read_optional(&device_path.join("mem_info_vram_used"))
            .ok()
            .flatten();

        match parse_drm_accelerator(
            &vendor,
            &device,
            &uevent,
            memory_total.as_deref(),
            memory_used.as_deref(),
        ) {
            Ok(Some(accelerator)) => accelerators.push(accelerator),
            Ok(None) => {}
            Err(detail) => warnings.push(format!("ignored DRM {name}: {detail}")),
        }
    }
    accelerators
}

fn current_cgroup_directory() -> Option<PathBuf> {
    let value = fs::read_to_string(PROC_CGROUP).ok()?;
    let relative = value.lines().find_map(|line| {
        let mut fields = line.splitn(3, ':');
        match (fields.next(), fields.next(), fields.next()) {
            (Some("0"), Some(""), Some(path)) => Some(path),
            _ => None,
        }
    })?;
    let candidate = Path::new(CGROUP_ROOT).join(relative.trim_start_matches('/'));
    candidate.is_dir().then_some(candidate)
}

fn read_cgroup_cpu_quota(directory: Option<&Path>, warnings: &mut Vec<String>) -> Option<f64> {
    let path = directory?.join("cpu.max");
    let value = match read_optional(&path) {
        Ok(value) => value?,
        Err(source) => {
            warnings.push(format!("could not read {}: {source}", path.display()));
            return None;
        }
    };
    match parse_cpu_quota(&value) {
        Ok(value) => value,
        Err(detail) => {
            warnings.push(format!("ignored {}: {detail}", path.display()));
            None
        }
    }
}

fn read_cgroup_memory(
    directory: Option<&Path>,
    warnings: &mut Vec<String>,
) -> Option<CgroupMemory> {
    let directory = directory?;
    let limit_path = directory.join("memory.max");
    let used_path = directory.join("memory.current");
    let limit = match read_optional(&limit_path) {
        Ok(Some(value)) if value.trim() == "max" => return None,
        Ok(Some(value)) => match value.trim().parse::<u64>() {
            Ok(value) => value,
            Err(source) => {
                warnings.push(format!("ignored {}: {source}", limit_path.display()));
                return None;
            }
        },
        Ok(None) => return None,
        Err(source) => {
            warnings.push(format!("could not read {}: {source}", limit_path.display()));
            return None;
        }
    };
    let used = match read_optional(&used_path) {
        Ok(Some(value)) => match value.trim().parse::<u64>() {
            Ok(value) => value,
            Err(source) => {
                warnings.push(format!("ignored {}: {source}", used_path.display()));
                return None;
            }
        },
        Ok(None) => return None,
        Err(source) => {
            warnings.push(format!("could not read {}: {source}", used_path.display()));
            return None;
        }
    };
    Some(CgroupMemory {
        limit_bytes: limit,
        used_bytes: used,
    })
}

fn read_optional(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(value) => Ok(Some(value)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(source),
    }
}

fn parse_key_value_lines(input: &str) -> BTreeMap<String, String> {
    input
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            Some((key.trim().to_ascii_lowercase(), value.trim().to_owned()))
        })
        .collect()
}

fn is_inference_feature(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "sse4_2"
            | "fma"
            | "f16c"
            | "avx"
            | "avx2"
            | "avx_vnni"
            | "avx512f"
            | "avx512_vnni"
            | "amx_bf16"
            | "amx_int8"
            | "neon"
            | "asimd"
            | "dotprod"
            | "i8mm"
            | "bf16"
            | "sve"
            | "sve2"
    )
}

fn all_ascii_digits(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_owned();
    }
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut value = bytes as f64;
    let mut unit = 0_usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const X86_CPUINFO: &str = r#"
processor   : 0
physical id : 0
core id     : 0
model name  : Tiny Test CPU
flags       : fpu sse4_2 avx avx2 fma

processor   : 1
physical id : 0
core id     : 0
model name  : Tiny Test CPU
flags       : fpu sse4_2 avx avx2 fma

processor   : 2
physical id : 0
core id     : 1
model name  : Tiny Test CPU
flags       : fpu sse4_2 avx avx2 fma

processor   : 3
physical id : 0
core id     : 1
model name  : Tiny Test CPU
flags       : fpu sse4_2 avx avx2 fma
"#;

    const MEMINFO: &str = r#"
MemTotal:       16000 kB
MemFree:         1000 kB
MemAvailable:    8000 kB
SwapTotal:       4000 kB
SwapFree:        3000 kB
"#;

    #[test]
    fn cpuinfo_reports_topology_and_relevant_inference_features() {
        let cpu = parse_cpuinfo(X86_CPUINFO, 2, Some(1.5), Some(1)).unwrap();

        assert_eq!(cpu.model, "Tiny Test CPU");
        assert_eq!(cpu.physical_cores, Some(2));
        assert_eq!(cpu.logical_cpus, 4);
        assert_eq!(cpu.available_parallelism, 2);
        assert_eq!(cpu.cgroup_quota_cpus, Some(1.5));
        assert_eq!(cpu.numa_nodes, Some(1));
        assert_eq!(cpu.features, ["avx", "avx2", "fma", "sse4_2"]);
    }

    #[test]
    fn cpuinfo_prefers_arm_hardware_name_over_numeric_processor_id() {
        let value = concat!(
            "processor : 0\nFeatures : fp asimd dotprod\n\n",
            "processor : 1\nFeatures : fp asimd dotprod\n\n",
            "Hardware : Tiny ARM Board\n"
        );
        let cpu = parse_cpuinfo(value, 2, None, None).unwrap();

        assert_eq!(cpu.model, "Tiny ARM Board");
        assert_eq!(cpu.logical_cpus, 2);
        assert_eq!(cpu.features, ["asimd", "dotprod"]);
    }

    #[test]
    fn meminfo_applies_cgroup_limit_to_effective_memory() {
        let cgroup = CgroupMemory {
            limit_bytes: 8 * 1024 * 1024,
            used_bytes: 2 * 1024 * 1024,
        };
        let memory = parse_meminfo(MEMINFO, Some(cgroup)).unwrap();

        assert_eq!(memory.host_total_bytes, 16_000 * 1024);
        assert_eq!(memory.host_available_bytes, 8_000 * 1024);
        assert_eq!(memory.effective_total_bytes, 8 * 1024 * 1024);
        assert_eq!(memory.effective_available_bytes, 6 * 1024 * 1024);
        assert_eq!(memory.swap_total_bytes, 4_000 * 1024);
        assert_eq!(memory.swap_available_bytes, 3_000 * 1024);
    }

    #[test]
    fn cpu_quota_parser_handles_limits_and_unlimited_groups() {
        assert_eq!(parse_cpu_quota("150000 100000\n").unwrap(), Some(1.5));
        assert_eq!(parse_cpu_quota("max 100000\n").unwrap(), None);
        assert!(parse_cpu_quota("150000 0\n").is_err());
    }

    #[test]
    fn os_release_prefers_pretty_name() {
        let value = "NAME=Tiny Linux\nPRETTY_NAME=\"Tiny Linux 1\"\n";
        assert_eq!(parse_os_release(value), Some("Tiny Linux 1".to_owned()));
    }

    #[test]
    fn mountinfo_selects_deepest_mount_and_decodes_spaces() {
        let value = concat!(
            "36 25 8:1 / / rw,relatime - ext4 /dev/sda1 rw\n",
            "40 36 8:2 / /home/yaya\\040data rw,relatime - xfs /dev/sdb2 rw\n"
        );
        let mount = parse_mountinfo(Path::new("/home/yaya data/models/qwen.gguf"), value).unwrap();

        assert_eq!(mount.mount_point, Path::new("/home/yaya data"));
        assert_eq!(mount.major_minor, "8:2");
        assert_eq!(mount.filesystem, "xfs");
        assert_eq!(mount.device, "/dev/sdb2");
    }

    #[test]
    fn drm_metadata_identifies_vendor_and_available_memory() {
        let accelerator = parse_drm_accelerator(
            "0x10de\n",
            "0x1eb8\n",
            "DRIVER=nvidia\nPCI_ID=10DE:1EB8\n",
            Some("1000\n"),
            Some("250\n"),
        )
        .unwrap()
        .unwrap();

        assert_eq!(accelerator.backend, "nvidia");
        assert!(accelerator.name.contains("nvidia"));
        assert_eq!(accelerator.vendor_id.as_deref(), Some("0x10de"));
        assert_eq!(accelerator.device_id.as_deref(), Some("0x1eb8"));
        assert_eq!(accelerator.memory_total_bytes, Some(1000));
        assert_eq!(accelerator.memory_available_bytes, Some(750));
    }

    #[test]
    fn drm_metadata_ignores_unknown_display_only_vendor() {
        let accelerator = parse_drm_accelerator(
            "0x1234\n",
            "0x1111\n",
            "DRIVER=bochs-drm\nPCI_ID=1234:1111\n",
            None,
            None,
        )
        .unwrap();

        assert!(accelerator.is_none());
    }

    #[test]
    fn byte_formatter_uses_binary_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1536), "1.5 KiB");
    }
}
