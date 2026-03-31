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
    "greeting": {
      "comment": "Shown on the home screen.",
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
    },
    "pending": {
      "localizations": {
        "en": {
          "stringUnit": {
            "state": "translated",
            "value": "Pending"
          }
        }
      }
    }
  }
}
"#;

    fs::write(path, xcstrings).unwrap();
}

fn write_strings_fixture(path: &std::path::Path) {
    let strings = r#"/* Greeting */
"greeting" = "Hello";
"pending" = "Pending";
"#;
    fs::write(path, strings).unwrap();
}

fn write_xliff_fixture(path: &std::path::Path) {
    let xliff = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="Base.lproj/Localizable.strings" source-language="en" target-language="fr" datatype="plaintext">
    <body>
      <trans-unit id="greeting" xml:space="preserve" resname="GREETING">
        <source>Hello</source>
        <target>Bonjour</target>
        <note>Shown on the home screen.</note>
      </trans-unit>
      <trans-unit id="pending" xml:space="preserve">
        <source>Pending</source>
      </trans-unit>
    </body>
  </file>
</xliff>
"#;
    fs::write(path, xliff).unwrap();
}

#[test]
fn test_convert_xcstrings_to_xliff() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("Localizable.xcstrings");
    let output = temp_dir.path().join("Localizable.xliff");
    write_xcstrings_fixture(&input);

    let out = langcodec_cmd()
        .args([
            "convert",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--output-lang",
            "fr",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let xml = fs::read_to_string(&output).unwrap();
    assert!(xml.contains(r#"target-language="fr""#));
    assert!(xml.contains("<target>Bonjour</target>"));
    assert!(xml.contains("<target/>"));
}

#[test]
fn test_convert_xliff_to_xcstrings() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("Localizable.xliff");
    let output = temp_dir.path().join("Localizable.xcstrings");
    write_xliff_fixture(&input);

    let out = langcodec_cmd()
        .args([
            "convert",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains(r#""sourceLanguage": "en""#));
    assert!(content.contains(r#""fr""#));
    assert!(content.contains(r#""pending""#));
}

#[test]
fn test_convert_strings_to_xliff_with_empty_target_language() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("en.strings");
    let output = temp_dir.path().join("Localizable.xliff");
    write_strings_fixture(&input);

    let out = langcodec_cmd()
        .args([
            "convert",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--source-language",
            "en",
            "--output-lang",
            "fr",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let xml = fs::read_to_string(&output).unwrap();
    assert!(xml.contains(r#"source-language="en""#));
    assert!(xml.contains(r#"target-language="fr""#));
    assert!(xml.contains("<target/>"));
}

#[test]
fn test_convert_xliff_to_android_strings() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("Localizable.xliff");
    let output = temp_dir.path().join("values-fr").join("strings.xml");
    write_xliff_fixture(&input);

    let out = langcodec_cmd()
        .args([
            "convert",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--output-lang",
            "fr",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let xml = fs::read_to_string(&output).unwrap();
    assert!(xml.contains(r#"name="greeting""#));
    assert!(xml.contains("Bonjour"));
    assert!(!xml.contains(r#"<string name="pending">"#));
}

#[test]
fn test_convert_xliff_requires_output_lang() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("Localizable.xcstrings");
    let output = temp_dir.path().join("Localizable.xliff");
    write_xcstrings_fixture(&input);

    let out = langcodec_cmd()
        .args([
            "convert",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(combined.contains("--output-lang"));
}

#[test]
fn test_convert_xliff_surfaces_ambiguous_source_language() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("translations.csv");
    let output = temp_dir.path().join("Localizable.xliff");
    fs::write(
        &input,
        "key,en,fr,de\nwelcome,Welcome,Bienvenue,Willkommen\n",
    )
    .unwrap();

    let out = langcodec_cmd()
        .args([
            "convert",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--output-lang",
            "fr",
        ])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(combined.contains("--source-language") || combined.contains("Could not infer"));
}

#[test]
fn test_view_reads_xliff_input() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("Localizable.xliff");
    write_xliff_fixture(&input);

    let out = langcodec_cmd()
        .args([
            "view",
            "--input",
            input.to_str().unwrap(),
            "--lang",
            "fr",
            "--keys-only",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("greeting"));
    assert!(stdout.contains("pending"));
}

#[test]
fn test_debug_reads_xliff_input() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("Localizable.xliff");
    let output = temp_dir.path().join("debug.json");
    write_xliff_fixture(&input);

    let out = langcodec_cmd()
        .args([
            "debug",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json = fs::read_to_string(&output).unwrap();
    assert!(json.contains(r#""language": "en""#));
    assert!(json.contains(r#""language": "fr""#));
    assert!(json.contains(r#""greeting""#));
}

#[test]
fn test_edit_rejects_xliff() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("Localizable.xliff");
    write_xliff_fixture(&input);

    let out = langcodec_cmd()
        .args([
            "edit",
            "set",
            "--inputs",
            input.to_str().unwrap(),
            "--key",
            "greeting",
            "--value",
            "Bonjour!",
        ])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(combined.contains("not supported by `edit`"));
}

#[test]
fn test_sync_rejects_xliff() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.xliff");
    let target = temp_dir.path().join("target.csv");
    write_xliff_fixture(&source);
    fs::write(&target, "key,en,fr\ngreeting,Hello,Bonjour\n").unwrap();

    let out = langcodec_cmd()
        .args([
            "sync",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(combined.contains("not supported by `sync`"));
}

#[test]
fn test_normalize_rejects_xliff() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("Localizable.xliff");
    write_xliff_fixture(&input);

    let out = langcodec_cmd()
        .args(["normalize", "--inputs", input.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(combined.contains("not supported by `normalize`"));
}
