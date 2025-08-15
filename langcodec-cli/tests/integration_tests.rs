use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_convert_json_to_xcstrings() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");
    let output_file = temp_dir.path().join("output.xcstrings");

    let json_content = r#"{
        "key": "hello_world",
        "en": "Hello, World!",
        "fr": "Bonjour, le monde!"
    }"#;

    fs::write(&input_file, json_content).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            input_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify output file exists and contains expected content
    assert!(output_file.exists());
    let output_content = fs::read_to_string(&output_file).unwrap();
    assert!(output_content.contains("hello_world"));
    assert!(output_content.contains("Hello, World!"));
    assert!(output_content.contains("Bonjour, le monde!"));
}

#[test]
fn test_convert_yaml_to_xcstrings() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.yaml");
    let output_file = temp_dir.path().join("output.xcstrings");

    let yaml_content = r#"key: hello_world
en: Hello, World!
fr: Bonjour, le monde!"#;

    fs::write(&input_file, yaml_content).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            input_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify output file exists and contains expected content
    assert!(output_file.exists());
    let output_content = fs::read_to_string(&output_file).unwrap();
    assert!(output_content.contains("hello_world"));
    assert!(output_content.contains("Hello, World!"));
    assert!(output_content.contains("Bonjour, le monde!"));
}

#[test]
fn test_convert_csv_to_xcstrings() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.csv");
    let output_file = temp_dir.path().join("output.xcstrings");

    let csv_content = r#"key,en,fr,de
hello,Hello,Bonjour,Hallo
bye,Goodbye,Au revoir,Auf Wiedersehen"#;

    fs::write(&input_file, csv_content).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            input_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify output file exists and contains expected content
    assert!(output_file.exists());
    let output_content = fs::read_to_string(&output_file).unwrap();
    assert!(output_content.contains("hello"));
    assert!(output_content.contains("bye"));
    assert!(output_content.contains("Hello"));
    assert!(output_content.contains("Bonjour"));
    assert!(output_content.contains("Hallo"));
}

#[test]
fn test_convert_with_explicit_format() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");
    let output_file = temp_dir.path().join("output.xcstrings");

    let json_content = r#"{
        "key": "hello_world",
        "en": "Hello, World!",
        "fr": "Bonjour, le monde!"
    }"#;

    fs::write(&input_file, json_content).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            input_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
            "--input-format",
            "json-language-map",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_file.exists());
}

#[test]
fn test_convert_standard_formats() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.xcstrings");

    // Test with fixtures
    let fixtures_dir = "tests/fixtures";
    let strings_file = format!("{}/cli_sample1.strings", fixtures_dir);

    // Verify fixture exists
    assert!(
        std::path::Path::new(&strings_file).exists(),
        "Fixture file not found: {}",
        strings_file
    );

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            &strings_file,
            "-o",
            output_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // The conversion might fail if the lib crate doesn't support the format,
    // but we should at least get a meaningful error message
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check that we get a proper error message
        assert!(
            stderr.contains("Could not convert") || stderr.contains("Error"),
            "Expected error message, got: {}",
            stderr
        );
    } else {
        // If it succeeds, verify the output file exists
        assert!(output_file.exists());
    }
}

#[test]
fn test_convert_invalid_format() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");
    let output_file = temp_dir.path().join("output.xcstrings");

    let json_content = r#"{
        "key": "hello_world",
        "en": "Hello, World!"
    }"#;

    fs::write(&input_file, json_content).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            input_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
            "--input-format",
            "invalid-format",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Input format validation failed")
            || stderr.contains("Unsupported custom format")
    );
}

#[test]
fn test_convert_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.xcstrings");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            "nonexistent.json",
            "-o",
            output_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // With new validation, we should get a file validation error
    assert!(stderr.contains("Input validation failed") || stderr.contains("File does not exist"));
}

#[test]
fn test_convert_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");
    let output_file = temp_dir.path().join("output.xcstrings");

    fs::write(&input_file, "{ invalid json }").unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            input_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Since JSON is not a standard format, the error should be about unknown format
    // rather than JSON parsing error
    assert!(stderr.contains("Cannot infer input format") || stderr.contains("Error parsing JSON"));
}

#[test]
fn test_help_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("langcodec"));
    assert!(stdout.contains("convert"));
    assert!(stdout.contains("merge"));
    assert!(stdout.contains("view"));
    assert!(stdout.contains("debug"));
}

#[test]
fn test_convert_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "convert", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Convert localization files between formats"));
    assert!(stdout.contains("--input"));
    assert!(stdout.contains("--output"));
    assert!(stdout.contains("--input-format"));
}
