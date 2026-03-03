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

fn write_xcstrings_multilang_fixture(path: &std::path::Path) {
    let xcstrings = r#"{
  "sourceLanguage": "en",
  "version": "1.0",
  "strings": {
    "needs_review_key": {
      "localizations": {
        "en": {
          "stringUnit": {
            "state": "needs_review",
            "value": "Needs review EN"
          }
        },
        "fr": {
          "stringUnit": {
            "state": "needs_review",
            "value": "Needs review FR"
          }
        }
      }
    },
    "translated_key": {
      "localizations": {
        "en": {
          "stringUnit": {
            "state": "translated",
            "value": "Hello"
          }
        },
        "fr": {
          "stringUnit": {
            "state": "translated",
            "value": "Bonjour"
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

#[test]
fn test_view_status_summary_uses_filtered_counts_without_lang() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("Localizable.xcstrings");
    write_xcstrings_multilang_fixture(&input_file);

    let output = langcodec_cmd()
        .args([
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--status",
            "needs_review",
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
        stdout.contains("=== Summary ==="),
        "Expected summary output. stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("Total languages: 2"),
        "Expected filtered language count in summary. stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("Total unique keys: 1"),
        "Expected filtered unique key count in summary. stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("en: 1 entries"),
        "Expected filtered per-language count for en. stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("fr: 1 entries"),
        "Expected filtered per-language count for fr. stdout: {}",
        stdout
    );
}

#[test]
fn test_view_status_rejects_blank_status_list() {
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
            ",",
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
        stderr.contains("No valid statuses were provided"),
        "Expected clear blank-status error. stderr: {}",
        stderr
    );
}
