use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_edit_set_add_update_remove_strings_in_place() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("en.strings");

    // Initial .strings content with one key
    let initial = r#"/* Greeting */
"hello" = "Hello";
"#;
    fs::write(&input_file, initial).unwrap();

    // Add a new key
    let out_add = Command::new("cargo")
        .args([
            "run",
            "--",
            "edit",
            "set",
            "-i",
            input_file.to_str().unwrap(),
            "-k",
            "welcome",
            "-v",
            "Welcome!",
        ])
        .output()
        .unwrap();
    assert!(
        out_add.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out_add.stderr)
    );

    let after_add = fs::read_to_string(&input_file).unwrap();
    assert!(after_add.contains("\"hello\""));
    assert!(after_add.contains("\"welcome\""));
    assert!(after_add.contains("\"Welcome!\""));

    // Update existing key
    let out_update = Command::new("cargo")
        .args([
            "run",
            "--",
            "edit",
            "set",
            "-i",
            input_file.to_str().unwrap(),
            "-k",
            "welcome",
            "-v",
            "Welcome back!",
            "--status",
            "needs_review",
        ])
        .output()
        .unwrap();
    assert!(
        out_update.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out_update.stderr)
    );

    let after_update = fs::read_to_string(&input_file).unwrap();
    assert!(after_update.contains("\"welcome\""));
    assert!(after_update.contains("Welcome back!"));

    // Remove by omitting value
    let out_remove = Command::new("cargo")
        .args([
            "run",
            "--",
            "edit",
            "set",
            "-i",
            input_file.to_str().unwrap(),
            "-k",
            "welcome",
        ])
        .output()
        .unwrap();
    assert!(
        out_remove.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out_remove.stderr)
    );

    let after_remove = fs::read_to_string(&input_file).unwrap();
    assert!(!after_remove.contains("\"welcome\""));
}

#[test]
fn test_edit_set_with_output_path() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("en.strings");
    let output_file = temp_dir.path().join("out.strings");

    let initial = r#"/* Greeting */
"hello" = "Hello";
"#;
    fs::write(&input_file, initial).unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "edit",
            "set",
            "-i",
            input_file.to_str().unwrap(),
            "-k",
            "new_key",
            "-v",
            "Value",
            "-o",
            output_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(output_file.exists());
    let out_content = fs::read_to_string(&output_file).unwrap();
    assert!(out_content.contains("\"new_key\""));
    assert!(out_content.contains("\"Value\""));
}

#[test]
fn test_main_help_lists_edit() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("edit"));
}

