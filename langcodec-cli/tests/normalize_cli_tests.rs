use std::process::Command;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

#[test]
fn test_main_help_lists_normalize() {
    let output = langcodec_cmd().args(["--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("normalize"));
}

#[test]
fn test_normalize_command_executes_successfully() {
    let output = langcodec_cmd().args(["normalize"]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "normalize command failed with stderr: {stderr}"
    );
    assert!(!stderr.contains("panicked at"), "normalize command panicked");
}
