use std::fs;
use std::process::Command;
use tempfile::TempDir;

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
fn test_normalize_requires_inputs_argument() {
    let output = langcodec_cmd().args(["normalize"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--inputs"));
}

#[test]
fn test_normalize_command_executes_successfully() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("en.strings");
    fs::write(&input, "\"a\" = \"A\";\n\"b\" = \"B\";\n").unwrap();

    let output = langcodec_cmd()
        .args(["normalize", "-i", input.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "normalize command failed with stderr: {stderr}"
    );
    assert!(
        !stderr.contains("panicked at"),
        "normalize command panicked"
    );
}

#[test]
fn test_normalize_check_fails_on_drift() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("en.strings");
    fs::write(&input, "\"z\" = \"%@\";\n\"a\" = \"A\";\n").unwrap();

    let output = langcodec_cmd()
        .args(["normalize", "-i", input.to_str().unwrap(), "--check"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("would change"),
        "expected output to mention drift; got: {combined}"
    );
}

#[test]
fn test_normalize_dry_run_does_not_write() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("en.strings");
    let before = "\"z\" = \"%@\";\n\"a\" = \"A\";\n";
    fs::write(&input, before).unwrap();

    let output = langcodec_cmd()
        .args(["normalize", "-i", input.to_str().unwrap(), "--dry-run"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after = fs::read_to_string(&input).unwrap();
    assert_eq!(after, before);
}

#[test]
fn test_normalize_output_written_even_when_unchanged() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("en.strings");
    let output_path = temp_dir.path().join("out").join("normalized.strings");
    fs::write(&input, "\"a\" = \"A\";\n\"b\" = \"B\";\n").unwrap();

    let output = langcodec_cmd()
        .args([
            "normalize",
            "-i",
            input.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_path.exists(), "expected output file to be created");

    let written = fs::read_to_string(&output_path).unwrap();
    assert!(written.contains("\"a\" = \"A\";"));
    assert!(written.contains("\"b\" = \"B\";"));
}

#[test]
fn test_normalize_check_and_dry_run_with_output_do_not_create_directories() {
    for mode in ["--check", "--dry-run"] {
        let temp_dir = TempDir::new().unwrap();
        let input = temp_dir.path().join("en.strings");
        let missing_parent = temp_dir.path().join("missing").join("nested");
        let output_path = missing_parent.join("out.strings");
        fs::write(&input, "\"z\" = \"%@\";\n\"a\" = \"A\";\n").unwrap();

        let output = langcodec_cmd()
            .args([
                "normalize",
                "-i",
                input.to_str().unwrap(),
                "-o",
                output_path.to_str().unwrap(),
                mode,
            ])
            .output()
            .unwrap();

        if mode == "--check" {
            assert!(
                !output.status.success(),
                "check mode should fail when drift is detected"
            );
        } else {
            assert!(
                output.status.success(),
                "dry-run failed with stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        assert!(
            !missing_parent.exists(),
            "mode {mode} unexpectedly created output directory"
        );
        assert!(
            !output_path.exists(),
            "mode {mode} unexpectedly created output file"
        );
    }
}
