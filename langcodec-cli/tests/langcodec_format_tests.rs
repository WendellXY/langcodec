use std::process::Command;

#[test]
fn test_langcodec_format_detection() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "view",
            "-i",
            "tests/fixtures/cli_sample.langcodec",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello_world"));
    assert!(stdout.contains("welcome_message"));
    assert!(stdout.contains("Hello, World!"));
    assert!(stdout.contains("Welcome to our app!"));
}

#[test]
fn test_langcodec_to_strings_conversion() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            "tests/fixtures/cli_sample.langcodec",
            "-o",
            "tests/fixtures/output_en.strings",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("tests/fixtures/output_en.strings").exists());

    // Clean up
    let _ = std::fs::remove_file("tests/fixtures/output_en.strings");
}

#[test]
fn test_langcodec_to_csv_conversion() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            "tests/fixtures/cli_sample.langcodec",
            "-o",
            "tests/fixtures/output.csv",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("tests/fixtures/output.csv").exists());

    // Clean up
    let _ = std::fs::remove_file("tests/fixtures/output.csv");
}

#[test]
fn test_langcodec_format_validation() {
    // Test with explicit format specification
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "convert",
            "-i",
            "tests/fixtures/cli_sample.langcodec",
            "-o",
            "tests/fixtures/output_test.strings",
            "--input-format",
            "langcodec-resource-array",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Clean up
    let _ = std::fs::remove_file("tests/fixtures/output_test.strings");
}
