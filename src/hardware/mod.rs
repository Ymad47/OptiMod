//! Host resource inspection and snapshot contracts.

use std::{
    fmt, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod linux;

pub const INSPECTION_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectionReport {
    pub schema_version: u16,
    pub host: HostSnapshot,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostSnapshot {
    pub operating_system: String,
    pub architecture: String,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub accelerators: Vec<AcceleratorSnapshot>,
    pub storage: StorageSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub model: String,
    pub physical_cores: Option<usize>,
    pub logical_cpus: usize,
    pub available_parallelism: usize,
    pub cgroup_quota_cpus: Option<f64>,
    pub features: Vec<String>,
    pub numa_nodes: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub host_total_bytes: u64,
    pub host_available_bytes: u64,
    pub effective_total_bytes: u64,
    pub effective_available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_available_bytes: u64,
    pub cgroup_limit_bytes: Option<u64>,
    pub cgroup_used_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceleratorSnapshot {
    pub backend: String,
    pub name: String,
    pub vendor_id: Option<String>,
    pub device_id: Option<String>,
    pub memory_total_bytes: Option<u64>,
    pub memory_available_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageSnapshot {
    pub path: PathBuf,
    pub filesystem: Option<String>,
    pub device: Option<String>,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub rotational: Option<bool>,
}

#[derive(Debug)]
pub enum InspectionError {
    UnsupportedPlatform,
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    InvalidData {
        input: &'static str,
        detail: String,
    },
}

impl InspectionError {
    pub(super) fn io(action: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            action,
            path: path.into(),
            source,
        }
    }

    pub(super) fn invalid(input: &'static str, detail: impl Into<String>) -> Self {
        Self::InvalidData {
            input,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for InspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(formatter, "host inspection currently supports Linux only")
            }
            Self::Io {
                action,
                path,
                source,
            } => write!(formatter, "failed to {action} {}: {source}", path.display()),
            Self::InvalidData { input, detail } => {
                write!(formatter, "invalid {input} data: {detail}")
            }
        }
    }
}

impl std::error::Error for InspectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::UnsupportedPlatform | Self::InvalidData { .. } => None,
        }
    }
}

pub fn inspect(path: &Path) -> Result<InspectionReport, InspectionError> {
    #[cfg(target_os = "linux")]
    {
        linux::inspect(path)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        Err(InspectionError::UnsupportedPlatform)
    }
}

pub fn render_human(report: &InspectionReport) -> String {
    #[cfg(target_os = "linux")]
    {
        linux::render_human(report)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = report;
        String::new()
    }
}
