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
    assert!(stdout.contains("tolgee"));
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
    assert!(stdout.contains("--ui"));
    assert!(stdout.contains(".strings"));
    assert!(stdout.contains("strings.xml"));
}

#[test]
fn test_translate_help_mentions_ui_flag() {
    let output = langcodec_cmd()
        .args(["translate", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--ui"));
    assert!(stdout.contains("auto"));
    assert!(stdout.contains("--tolgee"));
    assert!(stdout.contains("--tolgee-config"));
    assert!(stdout.contains("--tolgee-namespace"));
}

#[test]
fn test_tolgee_help_mentions_pull_and_push() {
    let output = langcodec_cmd().args(["tolgee", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pull"));
    assert!(stdout.contains("push"));
}

#[test]
fn test_tolgee_pull_help_mentions_namespace_flag() {
    let output = langcodec_cmd()
        .args(["tolgee", "pull", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--namespace"));
}

#[test]
fn test_tolgee_push_help_mentions_namespace_flag() {
    let output = langcodec_cmd()
        .args(["tolgee", "push", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--namespace"));
}
