use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

fn write_xcstrings_fixture(path: &std::path::Path) {
    let xcstrings = r#"{
  "sourceLanguage": "en",
  "version": "1.0",
  "strings": {
    "translated_key": {
      "localizations": {
        "en": {
          "stringUnit": {
            "state": "translated",
            "value": "Hello"
          }
        }
      }
    },
    "needs_review_key": {
      "localizations": {
        "en": {
          "stringUnit": {
            "state": "needs_review",
            "value": "Needs review"
          }
        }
      }
    }
  }
}
"#;

    fs::write(path, xcstrings).unwrap();
}

#[test]
fn test_view_status_filters_single_status_for_lang() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("Localizable.xcstrings");
    write_xcstrings_fixture(&input_file);

    let output = langcodec_cmd()
        .args([
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--lang",
            "en",
            "--status",
            "needs-review",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("needs_review_key"),
        "Expected filtered entry in output. stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("translated_key"),
        "Unexpected non-matching entry in output. stdout: {}",
        stdout
    );
}

#[test]
fn test_view_status_rejects_invalid_status() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("Localizable.xcstrings");
    write_xcstrings_fixture(&input_file);

    let output = langcodec_cmd()
        .args([
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--lang",
            "en",
            "--status",
            "not-a-status",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "CLI unexpectedly succeeded. stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid status") || stderr.contains("Unknown entry status"),
        "Expected invalid status error. stderr: {}",
        stderr
    );
}
