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

#[test]
fn scaffold_commands_fail_with_an_explicit_message() {
    let output = optimod(&["inspect"]);
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
