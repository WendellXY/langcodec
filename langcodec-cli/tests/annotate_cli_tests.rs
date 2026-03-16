use std::process::Command;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

#[test]
fn test_main_help_lists_annotate() {
    let output = langcodec_cmd().args(["--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("annotate"));
}

#[test]
fn test_annotate_help_mentions_source_root_flag() {
    let output = langcodec_cmd()
        .args(["annotate", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--source-root"));
    assert!(stdout.contains("--check"));
}
