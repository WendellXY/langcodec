use std::fs;
use std::process::Command;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

#[test]
fn test_langcodec_exclude_single_language() {
    // Create a test file with multiple languages
    let test_content = r#"key,en,fr,es
hello,Hello,Bonjour,Hola
world,World,Monde,Mundo"#;

    fs::write("test_multi.csv", test_content).expect("Failed to create test file");

    let output = langcodec_cmd()
        .args([
            "convert",
            "-i",
            "test_multi.csv",
            "-o",
            "test_filtered.langcodec",
            "--exclude-lang",
            "fr",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_filtered.langcodec").exists());

    // Validate the output structure and check that French is excluded
    let content =
        fs::read_to_string("test_filtered.langcodec").expect("Failed to read output file");
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output should be valid Resource array");

    let resources = resources.unwrap();
    assert_eq!(resources.len(), 2, "Should have 2 resources (en and es)");

    // Check that French is not present
    let languages: Vec<&str> = resources
        .iter()
        .map(|r| r.metadata.language.as_str())
        .collect();
    assert!(!languages.contains(&"fr"), "French should be excluded");
    assert!(languages.contains(&"en"), "English should be present");
    assert!(languages.contains(&"es"), "Spanish should be present");

    // Clean up
    let _ = fs::remove_file("test_multi.csv");
    let _ = fs::remove_file("test_filtered.langcodec");
}

#[test]
fn test_langcodec_exclude_multiple_languages() {
    // Create a test file with multiple languages
    let test_content = r#"key,en,fr,es,de
hello,Hello,Bonjour,Hola,Hallo
world,World,Monde,Mundo,Welt"#;

    fs::write("test_multi2.csv", test_content).expect("Failed to create test file");

    let output = langcodec_cmd()
        .args([
            "convert",
            "-i",
            "test_multi2.csv",
            "-o",
            "test_filtered2.langcodec",
            "--exclude-lang",
            "fr",
            "--exclude-lang",
            "de",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_filtered2.langcodec").exists());

    // Validate the output structure and check that French and German are excluded
    let content =
        fs::read_to_string("test_filtered2.langcodec").expect("Failed to read output file");
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output should be valid Resource array");

    let resources = resources.unwrap();
    assert_eq!(resources.len(), 2, "Should have 2 resources (en and es)");

    // Check that French and German are not present
    let languages: Vec<&str> = resources
        .iter()
        .map(|r| r.metadata.language.as_str())
        .collect();
    assert!(!languages.contains(&"fr"), "French should be excluded");
    assert!(!languages.contains(&"de"), "German should be excluded");
    assert!(languages.contains(&"en"), "English should be present");
    assert!(languages.contains(&"es"), "Spanish should be present");

    // Clean up
    let _ = fs::remove_file("test_multi2.csv");
    let _ = fs::remove_file("test_filtered2.langcodec");
}

#[test]
fn test_langcodec_no_exclusion() {
    // Test that when no exclusion is specified, all languages are included
    let test_content = r#"key,en,fr,es
hello,Hello,Bonjour,Hola
world,World,Monde,Mundo"#;

    fs::write("test_no_filter.csv", test_content).expect("Failed to create test file");

    let output = langcodec_cmd()
        .args([
            "convert",
            "-i",
            "test_no_filter.csv",
            "-o",
            "test_no_filtered.langcodec",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_no_filtered.langcodec").exists());

    // Validate that all languages are present
    let content =
        fs::read_to_string("test_no_filtered.langcodec").expect("Failed to read output file");
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output should be valid Resource array");

    let resources = resources.unwrap();
    assert_eq!(resources.len(), 3, "Should have 3 resources (en, fr, es)");

    // Check that all languages are present
    let languages: Vec<&str> = resources
        .iter()
        .map(|r| r.metadata.language.as_str())
        .collect();
    assert!(languages.contains(&"en"), "English should be present");
    assert!(languages.contains(&"fr"), "French should be present");
    assert!(languages.contains(&"es"), "Spanish should be present");

    // Clean up
    let _ = fs::remove_file("test_no_filter.csv");
    let _ = fs::remove_file("test_no_filtered.langcodec");
}

#[test]
fn test_langcodec_exclude_nonexistent_language() {
    // Test that excluding a language that doesn't exist doesn't cause issues
    let test_content = r#"key,en,fr
hello,Hello,Bonjour
world,World,Monde"#;

    fs::write("test_nonexistent.csv", test_content).expect("Failed to create test file");

    let output = langcodec_cmd()
        .args([
            "convert",
            "-i",
            "test_nonexistent.csv",
            "-o",
            "test_nonexistent_filtered.langcodec",
            "--exclude-lang",
            "es", // Spanish doesn't exist in the input
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_nonexistent_filtered.langcodec").exists());

    // Validate that all original languages are still present
    let content = fs::read_to_string("test_nonexistent_filtered.langcodec")
        .expect("Failed to read output file");
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output should be valid Resource array");

    let resources = resources.unwrap();
    assert_eq!(resources.len(), 2, "Should have 2 resources (en and fr)");

    // Check that all languages are present
    let languages: Vec<&str> = resources
        .iter()
        .map(|r| r.metadata.language.as_str())
        .collect();
    assert!(languages.contains(&"en"), "English should be present");
    assert!(languages.contains(&"fr"), "French should be present");

    // Clean up
    let _ = fs::remove_file("test_nonexistent.csv");
    let _ = fs::remove_file("test_nonexistent_filtered.langcodec");
}

#[test]
fn test_langcodec_exclude_all_languages() {
    // Test edge case: exclude all languages
    let test_content = r#"key,en,fr
hello,Hello,Bonjour
world,World,Monde"#;

    fs::write("test_exclude_all.csv", test_content).expect("Failed to create test file");

    let output = langcodec_cmd()
        .args([
            "convert",
            "-i",
            "test_exclude_all.csv",
            "-o",
            "test_exclude_all_filtered.langcodec",
            "--exclude-lang",
            "en",
            "--exclude-lang",
            "fr",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_exclude_all_filtered.langcodec").exists());

    // Validate that no resources are present
    let content = fs::read_to_string("test_exclude_all_filtered.langcodec")
        .expect("Failed to read output file");
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output should be valid Resource array");

    let resources = resources.unwrap();
    assert_eq!(
        resources.len(),
        0,
        "Should have 0 resources when all languages are excluded"
    );

    // Clean up
    let _ = fs::remove_file("test_exclude_all.csv");
    let _ = fs::remove_file("test_exclude_all_filtered.langcodec");
}
