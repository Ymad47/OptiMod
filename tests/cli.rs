use std::process::{Command, Output};

fn optimod(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_optimod"))
        .args(args)
        .output()
        .expect("optimod binary should start")
}

#[test]
fn help_lists_the_planned_top_level_commands() {
    let output = optimod(&["--help"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    for command in ["inspect", "model", "recommend", "benchmark", "report"] {
        assert!(
            stdout.contains(command),
            "help should list {command}; output was:\n{stdout}"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn inspect_json_describes_the_real_linux_host() {
    let output = optimod(&["inspect", "--json"]);
    assert!(
        output.status.success(),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("inspect output should be JSON");
    assert_eq!(report["schema_version"], 1);
    assert!(
        report["host"]["operating_system"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );
    assert!(
        report["host"]["cpu"]["logical_cpus"]
            .as_u64()
            .is_some_and(|v| v > 0)
    );
    assert!(
        report["host"]["memory"]["host_total_bytes"]
            .as_u64()
            .is_some_and(|v| v > 0)
    );
    assert!(report["host"]["storage"]["path"].as_str().is_some());
}

#[cfg(target_os = "linux")]
#[test]
fn inspect_human_output_names_each_resource_group() {
    let output = optimod(&["inspect"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    for heading in [
        "Operating system",
        "CPU",
        "Memory",
        "Accelerators",
        "Storage",
    ] {
        assert!(
            stdout.contains(heading),
            "output should contain {heading}; output was:\n{stdout}"
        );
    }
}

#[test]
fn unfinished_commands_fail_with_an_explicit_message() {
    let output = optimod(&["recommend", "qwen.gguf"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not implemented"),
        "scaffold status should be explicit; stderr was:\n{stderr}"
    );
}

#[test]
fn model_inspection_uses_a_nested_command() {
    let output = optimod(&["model", "inspect", "qwen.gguf"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("model inspect"));
    assert!(stderr.contains("not implemented"));
}
