use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

#[test]
fn test_strings_to_langcodec_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("test_simple.strings");
    let output = temp_dir.path().join("test_output.langcodec");

    let test_content = r#"/* Test localization */
"hello" = "Hello";
"world" = "World";"#;

    fs::write(&input, test_content).expect("Failed to create test file");

    let command_output = langcodec_cmd()
        .args(["convert", "-i"])
        .arg(&input)
        .args(["-o"])
        .arg(&output)
        .output()
        .expect("Failed to execute command");

    assert!(
        command_output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    assert!(output.exists());

    let content = fs::read_to_string(&output).expect("Failed to read output file");
    let resources: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output is not valid JSON array");

    let resources = resources.expect("Output should deserialize");
    assert!(
        !resources.is_empty(),
        "Output should contain at least one resource"
    );
}

#[test]
fn test_csv_to_langcodec_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("test_simple.csv");
    let output = temp_dir.path().join("test_output.csv.langcodec");

    let test_content = r#"key,en
hello,Hello
world,World"#;

    fs::write(&input, test_content).expect("Failed to create test file");

    let command_output = langcodec_cmd()
        .args(["convert", "-i"])
        .arg(&input)
        .args(["-o"])
        .arg(&output)
        .output()
        .expect("Failed to execute command");

    assert!(
        command_output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    assert!(output.exists());
}

#[test]
fn test_explicit_format_to_langcodec() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("test_explicit.strings");
    let output = temp_dir.path().join("test_output_explicit.langcodec");

    let test_content = r#"/* Test localization */
"hello" = "Hello";
"world" = "World";"#;

    fs::write(&input, test_content).expect("Failed to create test file");

    let command_output = langcodec_cmd()
        .args(["convert", "-i"])
        .arg(&input)
        .args(["-o"])
        .arg(&output)
        .args(["--input-format", "strings"])
        .output()
        .expect("Failed to execute command");

    assert!(
        command_output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    assert!(output.exists());
}

#[test]
fn test_langcodec_output_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("test_structure.strings");
    let output = temp_dir.path().join("test_structure_output.langcodec");

    let test_content = r#"/* Test localization */
"hello" = "Hello";"#;

    fs::write(&input, test_content).expect("Failed to create test file");

    let command_output = langcodec_cmd()
        .args(["convert", "-i"])
        .arg(&input)
        .args(["-o"])
        .arg(&output)
        .output()
        .expect("Failed to execute command");

    assert!(
        command_output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    let content = fs::read_to_string(&output).expect("Failed to read output file");

    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(
        resources.is_ok(),
        "Output should be valid Resource array: {:?}",
        resources.err()
    );

    let resources = resources.expect("Output should deserialize");
    assert!(!resources.is_empty(), "Should have at least one resource");

    for resource in &resources {
        let has_language = !resource.metadata.language.is_empty();
        let has_domain = !resource.metadata.domain.is_empty();
        assert!(
            has_language || has_domain,
            "Resource should have either language or domain, got language: '{}', domain: '{}'",
            resource.metadata.language,
            resource.metadata.domain
        );
        assert!(!resource.entries.is_empty(), "Resource should have entries");

        for entry in &resource.entries {
            assert!(!entry.id.is_empty(), "Entry should have id");
            match &entry.value {
                langcodec::Translation::Empty => {}
                langcodec::Translation::Singular(s) => {
                    assert!(!s.is_empty(), "Singular value should not be empty")
                }
                langcodec::Translation::Plural(p) => {
                    assert!(!p.id.is_empty(), "Plural id should not be empty")
                }
            }
        }
    }
}

#[test]
fn test_langcodec_pretty_printing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input = temp_dir.path().join("test_pretty.strings");
    let output = temp_dir.path().join("test_pretty_output.langcodec");

    let test_content = r#"/* Test localization */
"hello" = "Hello";"#;

    fs::write(&input, test_content).expect("Failed to create test file");

    let command_output = langcodec_cmd()
        .args(["convert", "-i"])
        .arg(&input)
        .args(["-o"])
        .arg(&output)
        .output()
        .expect("Failed to execute command");

    assert!(
        command_output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&command_output.stderr)
    );

    let content = fs::read_to_string(&output).expect("Failed to read output file");

    assert!(
        content.contains('\n'),
        "Output should be pretty-printed with newlines"
    );
    assert!(
        content.contains("  "),
        "Output should be pretty-printed with indentation"
    );

    assert!(
        content.starts_with("[\n"),
        "Output should start with '[' and newline"
    );
    assert!(
        content.ends_with("\n]"),
        "Output should end with newline and ']'"
    );
}
