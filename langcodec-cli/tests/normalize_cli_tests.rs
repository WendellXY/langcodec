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
fn test_normalize_check_detects_drift_across_standard_formats() {
    let temp_dir = TempDir::new().unwrap();
    let cases = [
        (
            "apple_strings",
            "en.strings",
            "\"z\" = \"%@\";\n\"a\" = \"A\";\n",
        ),
        (
            "android_xml",
            "strings.xml",
            r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="z">Hello %@</string>
    <string name="a">A</string>
</resources>
"#,
        ),
        ("csv", "translations.csv", "z,%@\na,A\n"),
        ("tsv", "translations.tsv", "z\t%@\na\tA\n"),
        (
            "xcstrings",
            "Localizable.xcstrings",
            r#"{
  "sourceLanguage": "en",
  "version": "1.0",
  "strings": {
    "z": {
      "localizations": {
        "en": {
          "stringUnit": {
            "state": "translated",
            "value": "Hello %@"
          }
        }
      }
    },
    "a": {
      "localizations": {
        "en": {
          "stringUnit": {
            "state": "translated",
            "value": "A"
          }
        }
      }
    }
  }
}
"#,
        ),
    ];

    for (label, filename, contents) in cases {
        let input = temp_dir.path().join(filename);
        fs::write(&input, contents).unwrap();

        let output = langcodec_cmd()
            .args(["normalize", "-i", input.to_str().unwrap(), "--check"])
            .output()
            .unwrap();

        assert!(
            !output.status.success(),
            "{label} should report drift in --check mode"
        );
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            combined.contains("would change"),
            "{label} expected drift message, got: {combined}"
        );
    }
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

#[test]
fn test_normalize_rejects_output_with_multiple_inputs() {
    let temp_dir = TempDir::new().unwrap();
    let input_a = temp_dir.path().join("a.strings");
    let input_b = temp_dir.path().join("b.strings");
    let output_path = temp_dir.path().join("out.strings");
    fs::write(&input_a, "\"a\" = \"A\";\n").unwrap();
    fs::write(&input_b, "\"b\" = \"B\";\n").unwrap();

    let output = langcodec_cmd()
        .args([
            "normalize",
            "-i",
            input_a.to_str().unwrap(),
            input_b.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("--output cannot be used with multiple input files"),
        "expected --output multi-input rejection; got: {combined}"
    );
}

#[test]
fn test_normalize_continue_on_error_aggregates_and_returns_non_zero() {
    let temp_dir = TempDir::new().unwrap();
    let good = temp_dir.path().join("good.strings");
    let bad = temp_dir.path().join("bad.txt");
    fs::write(&good, "\"z\" = \"%@\";\n\"a\" = \"A\";\n").unwrap();
    fs::write(&bad, "not a supported localization format").unwrap();

    let output = langcodec_cmd()
        .args([
            "normalize",
            "-i",
            bad.to_str().unwrap(),
            good.to_str().unwrap(),
            "--continue-on-error",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero when at least one file fails"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("Summary: processed 2; success: 1; failed: 1"),
        "expected summary counts, got: {combined}"
    );
    assert!(
        combined.contains("✅ Normalized:"),
        "expected successful file processing to continue, got: {combined}"
    );
}

#[test]
fn test_normalize_continue_on_error_counts_literal_missing_input_in_summary() {
    let temp_dir = TempDir::new().unwrap();
    let good = temp_dir.path().join("good.strings");
    let missing = temp_dir.path().join("missing.strings");
    fs::write(&good, "\"z\" = \"%@\";\n\"a\" = \"A\";\n").unwrap();

    let output = langcodec_cmd()
        .args([
            "normalize",
            "-i",
            missing.to_str().unwrap(),
            good.to_str().unwrap(),
            "--continue-on-error",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero when at least one file fails"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("Input file does not exist"),
        "expected missing-input error, got: {combined}"
    );
    assert!(
        combined.contains("Summary: processed 2; success: 1; failed: 1"),
        "expected coherent summary counts, got: {combined}"
    );
}
