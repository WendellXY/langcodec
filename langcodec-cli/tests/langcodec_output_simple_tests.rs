use std::fs;
use std::process::Command;

#[test]
fn test_strings_to_langcodec_basic() {
    // Test the basic functionality by creating a simple test file
    let test_content = r#"/* Test localization */
"hello" = "Hello";
"world" = "World";"#;

    fs::write("test_simple.strings", test_content).expect("Failed to create test file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            "test_simple.strings",
            "-o",
            "test_output.langcodec",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_output.langcodec").exists());

    // Validate the output structure
    let content = fs::read_to_string("test_output.langcodec").expect("Failed to read output file");
    let resources: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output is not valid JSON array");

    let resources = resources.unwrap();
    assert!(
        !resources.is_empty(),
        "Output should contain at least one resource"
    );

    // Clean up
    let _ = fs::remove_file("test_simple.strings");
    let _ = fs::remove_file("test_output.langcodec");
}

#[test]
fn test_csv_to_langcodec_basic() {
    // Test CSV to langcodec conversion
    let test_content = r#"key,en
hello,Hello
world,World"#;

    fs::write("test_simple.csv", test_content).expect("Failed to create test file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            "test_simple.csv",
            "-o",
            "test_output.csv.langcodec",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_output.csv.langcodec").exists());

    // Clean up
    let _ = fs::remove_file("test_simple.csv");
    let _ = fs::remove_file("test_output.csv.langcodec");
}

#[test]
fn test_explicit_format_to_langcodec() {
    // Test with explicit format specification
    let test_content = r#"/* Test localization */
"hello" = "Hello";
"world" = "World";"#;

    fs::write("test_explicit.strings", test_content).expect("Failed to create test file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            "test_explicit.strings",
            "-o",
            "test_output_explicit.langcodec",
            "--input-format",
            "strings",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_output_explicit.langcodec").exists());

    // Clean up
    let _ = fs::remove_file("test_explicit.strings");
    let _ = fs::remove_file("test_output_explicit.langcodec");
}

#[test]
fn test_langcodec_output_structure() {
    // Test that the output has the correct structure
    let test_content = r#"/* Test localization */
"hello" = "Hello";"#;

    fs::write("test_structure.strings", test_content).expect("Failed to create test file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            "test_structure.strings",
            "-o",
            "test_structure_output.langcodec",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content =
        fs::read_to_string("test_structure_output.langcodec").expect("Failed to read output file");

    // Parse as actual Resource objects to validate structure
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(
        resources.is_ok(),
        "Output should be valid Resource array: {:?}",
        resources.err()
    );

    let resources = resources.unwrap();
    assert!(!resources.is_empty(), "Should have at least one resource");

    // Validate each resource has required fields
    for resource in &resources {
        assert!(
            !resource.metadata.language.is_empty(),
            "Resource should have language"
        );
        assert!(
            !resource.metadata.domain.is_empty(),
            "Resource should have domain"
        );
        assert!(!resource.entries.is_empty(), "Resource should have entries");

        // Validate each entry has required fields
        for entry in &resource.entries {
            assert!(!entry.id.is_empty(), "Entry should have id");
            // Value should be either Singular or Plural
            match &entry.value {
                langcodec::Translation::Singular(s) => {
                    assert!(!s.is_empty(), "Singular value should not be empty")
                }
                langcodec::Translation::Plural(p) => {
                    assert!(!p.id.is_empty(), "Plural id should not be empty")
                }
            }
        }
    }

    // Clean up
    let _ = fs::remove_file("test_structure.strings");
    let _ = fs::remove_file("test_structure_output.langcodec");
}

#[test]
fn test_langcodec_pretty_printing() {
    // Test that the output is pretty-printed
    let test_content = r#"/* Test localization */
"hello" = "Hello";"#;

    fs::write("test_pretty.strings", test_content).expect("Failed to create test file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            "test_pretty.strings",
            "-o",
            "test_pretty_output.langcodec",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content =
        fs::read_to_string("test_pretty_output.langcodec").expect("Failed to read output file");

    // Check that the output is pretty-printed (contains newlines and proper indentation)
    assert!(
        content.contains('\n'),
        "Output should be pretty-printed with newlines"
    );
    assert!(
        content.contains("  "),
        "Output should be pretty-printed with indentation"
    );

    // Should start with '[\n' and end with '\n]' for pretty-printed JSON array
    assert!(
        content.starts_with("[\n"),
        "Output should start with '[' and newline"
    );
    assert!(
        content.ends_with("\n]"),
        "Output should end with newline and ']'"
    );

    // Clean up
    let _ = fs::remove_file("test_pretty.strings");
    let _ = fs::remove_file("test_pretty_output.langcodec");
}
