use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

fn combined_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn write_translated_catalog(path: &Path) {
    fs::write(
        path,
        r#"{
  "sourceLanguage" : "en",
  "version" : "1.0",
  "strings" : {
    "welcome" : {
      "localizations" : {
        "en" : {
          "stringUnit" : {
            "state" : "translated",
            "value" : "Welcome"
          }
        },
        "fr" : {
          "stringUnit" : {
            "state" : "translated",
            "value" : "Bienvenue"
          }
        }
      }
    }
  }
}"#,
    )
    .unwrap();
}

fn write_manual_comment_catalog(path: &Path) {
    fs::write(
        path,
        r#"{
  "sourceLanguage" : "en",
  "version" : "1.0",
  "strings" : {
    "welcome" : {
      "comment" : "Shown on the home screen welcome banner.",
      "shouldTranslate" : true,
      "localizations" : {
        "en" : {
          "stringUnit" : {
            "state" : "translated",
            "value" : "Welcome"
          }
        }
      }
    }
  }
}"#,
    )
    .unwrap();
}

#[test]
fn translate_uses_globbed_sources_from_langcodec_toml_end_to_end() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    let feature_a = project_root.join("Modules").join("FeatureA");
    let feature_b = project_root.join("Modules").join("FeatureB");
    fs::create_dir_all(&feature_a).unwrap();
    fs::create_dir_all(&feature_b).unwrap();

    let first = feature_a.join("Localizable.xcstrings");
    let second = feature_b.join("Localizable.xcstrings");
    write_translated_catalog(&first);
    write_translated_catalog(&second);

    let original_first = fs::read_to_string(&first).unwrap();
    let original_second = fs::read_to_string(&second).unwrap();

    let config = project_root.join("langcodec.toml");
    fs::write(
        &config,
        r#"[openai]
model = "gpt-5.4"

[translate.input]
sources = ["Modules/*/Localizable.xcstrings"]
lang = "en"

[translate.output]
lang = ["fr"]
"#,
    )
    .unwrap();

    let output = langcodec_cmd()
        .current_dir(project_root)
        .env("OPENAI_API_KEY", "test-key")
        .args([
            "translate",
            "--config",
            config.to_str().unwrap(),
            "--dry-run",
            "--ui",
            "plain",
        ])
        .output()
        .unwrap();

    let combined = combined_output(&output);
    assert!(output.status.success(), "combined output: {combined}");
    assert!(combined.contains("Running 2 translate jobs in parallel from config"));
    assert!(combined.contains(first.to_str().unwrap()));
    assert!(combined.contains(second.to_str().unwrap()));
    assert!(
        count_occurrences(&combined, "Queued for translation: 0") >= 2,
        "expected both config-matched runs to report zero queued jobs, got: {combined}"
    );
    assert_eq!(fs::read_to_string(&first).unwrap(), original_first);
    assert_eq!(fs::read_to_string(&second).unwrap(), original_second);
}

#[test]
fn annotate_uses_globbed_inputs_from_langcodec_toml_end_to_end() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    let sources_dir = project_root.join("Sources");
    let app_dir = project_root.join("App").join("Resources");
    let module_dir = project_root.join("Modules").join("Feature");
    fs::create_dir_all(&sources_dir).unwrap();
    fs::create_dir_all(&app_dir).unwrap();
    fs::create_dir_all(&module_dir).unwrap();

    let first = app_dir.join("Localizable.xcstrings");
    let second = module_dir.join("Localizable.xcstrings");
    write_manual_comment_catalog(&first);
    write_manual_comment_catalog(&second);

    let original_first = fs::read_to_string(&first).unwrap();
    let original_second = fs::read_to_string(&second).unwrap();

    let config = project_root.join("langcodec.toml");
    fs::write(
        &config,
        r#"[openai]
model = "gpt-5.4"

[annotate]
inputs = ["*/**/Localizable.xcstrings"]
source_roots = ["Sources"]
"#,
    )
    .unwrap();

    let output = langcodec_cmd()
        .current_dir(project_root)
        .env("OPENAI_API_KEY", "test-key")
        .args([
            "annotate",
            "--config",
            config.to_str().unwrap(),
            "--dry-run",
            "--ui",
            "plain",
        ])
        .output()
        .unwrap();

    let combined = combined_output(&output);
    assert!(output.status.success(), "combined output: {combined}");
    assert_eq!(
        count_occurrences(&combined, "No entries require annotation updates."),
        2,
        "expected both config-matched catalogs to be processed, got: {combined}"
    );
    assert_eq!(fs::read_to_string(&first).unwrap(), original_first);
    assert_eq!(fs::read_to_string(&second).unwrap(), original_second);
}
