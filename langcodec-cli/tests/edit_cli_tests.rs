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
fn test_edit_set_dry_run_add_does_not_write() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("en.strings");

    let initial = r#"/* Greeting */
"hello" = "Hello";
"#;
    fs::write(&input_file, initial).unwrap();

    let output = Command::new("cargo")
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
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DRY-RUN"));
    let after = fs::read_to_string(&input_file).unwrap();
    assert_eq!(after, initial);
}

#[test]
fn test_edit_set_dry_run_update_does_not_write() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("en.strings");

    let initial = r#"/* Greeting */
"hello" = "Hello";
"#;
    fs::write(&input_file, initial).unwrap();

    // First, add welcome so we can dry-run update
    let _ = Command::new("cargo")
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

    let before = fs::read_to_string(&input_file).unwrap();

    let output = Command::new("cargo")
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
            "Welcome again!",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DRY-RUN"));
    let after = fs::read_to_string(&input_file).unwrap();
    assert_eq!(after, before);
}

#[test]
fn test_edit_set_dry_run_remove_does_not_write() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("en.strings");

    let initial = r#"/* Greeting */
"hello" = "Hello";
"welcome" = "Welcome!";
"#;
    fs::write(&input_file, initial).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "edit",
            "set",
            "-i",
            input_file.to_str().unwrap(),
            "-k",
            "welcome",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DRY-RUN"));
    let after = fs::read_to_string(&input_file).unwrap();
    assert_eq!(after, initial);
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

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
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

#[test]
fn test_edit_set_multiple_files() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("a.strings");
    let file2 = temp_dir.path().join("b.strings");
    fs::write(&file1, "\"hello\" = \"Hello\";\n").unwrap();
    fs::write(&file2, "\"hello\" = \"Hello\";\n").unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "edit",
            "set",
            "-i",
            file1.to_str().unwrap(),
            "-i",
            file2.to_str().unwrap(),
            "-k",
            "welcome",
            "-v",
            "Welcome!",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let c1 = fs::read_to_string(&file1).unwrap();
    let c2 = fs::read_to_string(&file2).unwrap();
    assert!(c1.contains("\"welcome\""));
    assert!(c2.contains("\"welcome\""));
}

#[test]
fn test_edit_set_glob_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();
    let f1 = dir.join("a.strings");
    let f2 = dir.join("b.strings");
    fs::write(&f1, "\"hello\" = \"Hello\";\n").unwrap();
    fs::write(&f2, "\"hello\" = \"Hello\";\n").unwrap();

    let pattern = format!("{}/*.strings", dir.to_string_lossy());
    let out = Command::new("cargo")
        .args([
            "run", "--", "edit", "set", "-i", &pattern, "-k", "added", "-v", "Yes",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(fs::read_to_string(&f1).unwrap().contains("\"added\""));
    assert!(fs::read_to_string(&f2).unwrap().contains("\"added\""));
}

#[test]
fn test_edit_set_multiple_inputs_with_output_is_error() {
    let temp_dir = TempDir::new().unwrap();
    let f1 = temp_dir.path().join("a.strings");
    let f2 = temp_dir.path().join("b.strings");
    let out_file = temp_dir.path().join("out.strings");
    fs::write(&f1, "\"hello\" = \"Hello\";\n").unwrap();
    fs::write(&f2, "\"hello\" = \"Hello\";\n").unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "edit",
            "set",
            "-i",
            f1.to_str().unwrap(),
            "-i",
            f2.to_str().unwrap(),
            "-k",
            "x",
            "-v",
            "y",
            "-o",
            out_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "expected failure when using --output with multiple inputs"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cannot be used with multiple input files"));
}

#[test]
fn test_edit_set_continue_on_error() {
    let temp_dir = TempDir::new().unwrap();
    let good = temp_dir.path().join("good.strings");
    let bad = temp_dir.path().join("missing.strings");
    fs::write(&good, "\"hello\" = \"Hello\";\n").unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "edit",
            "set",
            "-i",
            good.to_str().unwrap(),
            "-i",
            bad.to_str().unwrap(),
            "-k",
            "welcome",
            "-v",
            "Welcome!",
            "--continue-on-error",
        ])
        .output()
        .unwrap();

    // Expect non-zero (some files failed), but the good file should be updated
    assert!(
        !out.status.success(),
        "expected non-zero exit when some files fail"
    );
    let updated = fs::read_to_string(&good).unwrap();
    assert!(updated.contains("\"welcome\""));
}
