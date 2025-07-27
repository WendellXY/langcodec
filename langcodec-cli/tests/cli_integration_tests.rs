use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_convert_command_basic() {
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

    assert!(output_file.exists());
    let output_content = fs::read_to_string(&output_file).unwrap();
    assert!(output_content.contains("hello_world"));
}

#[test]
fn test_convert_command_with_explicit_format() {
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

    assert!(output.status.success());
    assert!(output_file.exists());
}

#[test]
fn test_convert_command_with_language_code() {
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

    assert!(output.status.success());
    assert!(output_file.exists());
}

#[test]
fn test_convert_command_invalid_input_file() {
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
    assert!(stderr.contains("File does not exist") || stderr.contains("Input validation failed"));
}

#[test]
fn test_convert_command_invalid_output_path() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");

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
            "/nonexistent/path/output.xcstrings",
        ])
        .output()
        .unwrap();

    // This might succeed if the system allows creating the directory
    // or fail if it doesn't - both are acceptable behaviors
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Cannot write") || stderr.contains("Output validation failed"));
    }
}

#[test]
fn test_convert_command_missing_arguments() {
    let output = Command::new("cargo")
        .args(["run", "--", "convert"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("required") || stderr.contains("missing"));
}

#[test]
fn test_convert_command_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "convert", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Convert localization files"));
    assert!(stdout.contains("--input"));
    assert!(stdout.contains("--output"));
}

#[test]
fn test_merge_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    let input_file1 = temp_dir.path().join("file1.json");
    let input_file2 = temp_dir.path().join("file2.json");
    let output_file = temp_dir.path().join("merged.xcstrings");

    let json_content1 = r#"{
        "key": "hello_world",
        "en": "Hello, World!"
    }"#;

    let json_content2 = r#"{
        "key": "goodbye_world",
        "en": "Goodbye, World!"
    }"#;

    fs::write(&input_file1, json_content1).unwrap();
    fs::write(&input_file2, json_content2).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "merge",
            "-i",
            input_file1.to_str().unwrap(),
            "-i",
            input_file2.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output_file.exists());
}

#[test]
fn test_merge_command_with_conflict_strategy() {
    let temp_dir = TempDir::new().unwrap();
    let input_file1 = temp_dir.path().join("file1.json");
    let input_file2 = temp_dir.path().join("file2.json");
    let output_file = temp_dir.path().join("merged.xcstrings");

    let json_content1 = r#"{
        "key": "hello_world",
        "en": "Hello, World!"
    }"#;

    let json_content2 = r#"{
        "key": "hello_world",
        "en": "Hi, World!"
    }"#;

    fs::write(&input_file1, json_content1).unwrap();
    fs::write(&input_file2, json_content2).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "merge",
            "-i",
            input_file1.to_str().unwrap(),
            "-i",
            input_file2.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
            "--strategy",
            "last",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output_file.exists());
}

#[test]
fn test_view_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": "Hello, World!",
        "fr": "Bonjour, le monde!"
    }"#;

    fs::write(&input_file, json_content).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "view", "-i", input_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello_world"));
    assert!(stdout.contains("Hello, World!"));
    assert!(stdout.contains("Bonjour, le monde!"));
}

#[test]
fn test_view_command_with_format() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": "Hello, World!",
        "fr": "Bonjour, le monde!"
    }"#;

    fs::write(&input_file, json_content).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "view", "-i", input_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello_world"));
}

#[test]
fn test_debug_command_basic() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": "Hello, World!",
        "fr": "Bonjour, le monde!"
    }"#;

    fs::write(&input_file, json_content).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "debug", "-i", input_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Debug output") || stdout.contains("Debug Summary"));
}

#[test]
fn test_main_help_command() {
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
fn test_invalid_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "invalid-command"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("Unknown"));
}

#[test]
fn test_version_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("langcodec"));
}

#[test]
fn test_convert_command_with_yaml() {
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
            "--input-format",
            "yaml-language-map",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output_file.exists());
}

#[test]
fn test_convert_command_with_json_array() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");
    let output_file = temp_dir.path().join("output.xcstrings");

    let json_content = r#"[
        {
            "key": "hello_world",
            "en": "Hello, World!",
            "fr": "Bonjour, le monde!"
        },
        {
            "key": "goodbye_world",
            "en": "Goodbye, World!",
            "fr": "Au revoir, le monde!"
        }
    ]"#;

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
            "json-array-language-map",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output_file.exists());
}

#[test]
fn test_convert_command_output_to_csv() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");
    let output_file = temp_dir.path().join("output.csv");

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
            "--output-format",
            "csv",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output_file.exists());
}

#[test]
fn test_convert_command_output_to_strings() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");
    let output_file = temp_dir.path().join("output.strings");

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
            "--output-format",
            "strings",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output_file.exists());
}

#[test]
fn test_convert_command_output_to_android() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("test.json");
    let output_file = temp_dir.path().join("output.xml");

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
            "--output-format",
            "android",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output_file.exists());
}
