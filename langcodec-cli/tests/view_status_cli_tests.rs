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

fn write_xcstrings_partial_match_fixture(path: &std::path::Path) {
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
            "state": "translated",
            "value": "Besoin de revision FR"
          }
        }
      }
    }
  }
}
"#;

    fs::write(path, xcstrings).unwrap();
}

fn write_android_strings_fixture(path: &std::path::Path) {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="translated_key">Hello</string>
    <string name="needs_review_key">Needs review</string>
</resources>
"#;

    fs::write(path, xml).unwrap();
}

#[test]
fn test_view_status_strict_rejects_android_strings_without_status_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("strings.xml");
    write_android_strings_fixture(&input_file);

    let output = langcodec_cmd()
        .args([
            "--strict",
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--lang",
            "en",
            "--status",
            "translated",
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
        stderr.contains("explicit status metadata"),
        "Expected strict status metadata guard error. stderr: {}",
        stderr
    );
}

#[test]
fn test_view_status_strict_allows_xcstrings_with_status_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("Localizable.xcstrings");
    write_xcstrings_fixture(&input_file);

    let output = langcodec_cmd()
        .args([
            "--strict",
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--lang",
            "en",
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
        stdout.contains("needs_review_key"),
        "Expected filtered entry in output. stdout: {}",
        stdout
    );
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

#[test]
fn test_view_keys_only_with_lang_prints_key_per_line() {
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
            "needs_review",
            "--keys-only",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(
        lines.contains(&"needs_review_key"),
        "Expected raw key line in output. stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("Entry 1:"),
        "Expected keys-only output without verbose entry headings. stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("Status:"),
        "Expected keys-only output without status lines. stdout: {}",
        stdout
    );
    assert!(
        !lines.contains(&"translated_key"),
        "Expected non-matching keys to be excluded. stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("Processing resources..."),
        "Expected keys-only output without preamble lines. stdout: {}",
        stdout
    );
}

#[test]
fn test_view_keys_only_without_lang_prints_lang_tab_key() {
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
            "--keys-only",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(
        lines.contains(&"en\tneeds_review_key"),
        "Expected `lang<TAB>key` line for en. stdout: {}",
        stdout
    );
    assert!(
        lines.contains(&"fr\tneeds_review_key"),
        "Expected `lang<TAB>key` line for fr. stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("=== Summary ==="),
        "Expected keys-only output without summary block. stdout: {}",
        stdout
    );
    assert!(
        !lines.contains(&"en\ttranslated_key"),
        "Expected non-matching en key to be excluded. stdout: {}",
        stdout
    );
    assert!(
        !lines.contains(&"fr\ttranslated_key"),
        "Expected non-matching fr key to be excluded. stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("Processing resources..."),
        "Expected keys-only output without preamble lines. stdout: {}",
        stdout
    );
}

#[test]
fn test_view_status_json_outputs_entries_payload() {
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
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Expected JSON output. parse error: {e}. stdout: {stdout}"));

    assert_eq!(payload["summary"]["total_matches"], 2);
    assert_eq!(payload["summary"]["statuses"]["needs_review"], 2);
    let languages = payload["summary"]["languages"].as_array().unwrap();
    assert_eq!(
        languages.len(),
        2,
        "Expected 2 languages. payload: {payload}"
    );
    assert!(languages.iter().any(|lang| lang == "en"));
    assert!(languages.iter().any(|lang| lang == "fr"));

    let entries = payload["entries"].as_array().unwrap();
    assert_eq!(
        entries.len(),
        2,
        "Expected only filtered entries. payload: {payload}"
    );

    let en_entry = entries
        .iter()
        .find(|entry| entry["lang"] == "en")
        .expect("Expected English entry in JSON payload");
    assert_eq!(en_entry["key"], "needs_review_key");
    assert_eq!(en_entry["status"], "needs_review");
    assert_eq!(en_entry["type"], "singular");
    assert_eq!(en_entry["value"], "Needs review EN");
}

#[test]
fn test_view_status_json_keys_only_outputs_keys_payload() {
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
            "--keys-only",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Expected JSON output. parse error: {e}. stdout: {stdout}"));

    assert_eq!(payload["summary"]["total_matches"], 2);
    assert_eq!(payload["summary"]["statuses"]["needs_review"], 2);
    assert!(
        payload.get("entries").is_none(),
        "Expected keys-only payload"
    );

    let keys = payload["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 2);
    assert!(
        keys.iter()
            .any(|item| item["lang"] == "en" && item["key"] == "needs_review_key")
    );
    assert!(
        keys.iter()
            .any(|item| item["lang"] == "fr" && item["key"] == "needs_review_key")
    );
}

#[test]
fn test_view_status_json_excludes_zero_match_languages_in_summary() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("Localizable.xcstrings");
    write_xcstrings_partial_match_fixture(&input_file);

    let output = langcodec_cmd()
        .args([
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--status",
            "needs_review",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Expected JSON output. parse error: {e}. stdout: {stdout}"));

    let languages = payload["summary"]["languages"].as_array().unwrap();
    assert_eq!(languages.len(), 1);
    assert_eq!(languages[0], "en");
}

#[test]
fn test_view_status_json_keys_only_lang_uses_consistent_object_schema() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("Localizable.xcstrings");
    write_xcstrings_multilang_fixture(&input_file);

    let output = langcodec_cmd()
        .args([
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--lang",
            "en",
            "--status",
            "needs_review",
            "--keys-only",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Expected JSON output. parse error: {e}. stdout: {stdout}"));

    let keys = payload["keys"].as_array().unwrap();
    assert!(!keys.is_empty(), "Expected at least one key object");
    assert!(keys.iter().all(|item| item["lang"] == "en"));
    assert!(keys.iter().all(|item| item["key"] == "needs_review_key"));
}

#[test]
fn test_view_status_json_with_check_plurals_keeps_stdout_json() {
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
            "--json",
            "--check-plurals",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _payload: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Expected JSON output. parse error: {e}. stdout: {stdout}"));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Plural validation passed"),
        "Expected plural validation success in stderr. stderr: {}",
        stderr
    );
}

#[test]
fn test_view_status_text_excludes_zero_match_languages_from_output_and_summary() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("Localizable.xcstrings");
    write_xcstrings_partial_match_fixture(&input_file);

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
        stdout.contains("Language: en"),
        "Expected matched language in output. stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("Language: fr"),
        "Expected zero-match language to be excluded. stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("Total languages: 1"),
        "Expected summary to count only matching languages. stdout: {}",
        stdout
    );
}
