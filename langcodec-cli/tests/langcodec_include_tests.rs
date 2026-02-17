use std::fs;
use std::process::Command;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

#[test]
fn test_langcodec_include_single_language() {
    // Create a test file with multiple languages
    let test_content = r#"key,en,fr,es
hello,Hello,Bonjour,Hola
world,World,Monde,Mundo"#;

    fs::write("test_include_multi.csv", test_content).expect("Failed to create test file");

    let output = langcodec_cmd()
        .args([
            "convert",
            "-i",
            "test_include_multi.csv",
            "-o",
            "test_include_filtered.langcodec",
            "--include-lang",
            "en",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_include_filtered.langcodec").exists());

    // Validate the output structure and check that only English is included
    let content =
        fs::read_to_string("test_include_filtered.langcodec").expect("Failed to read output file");
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output should be valid Resource array");

    let resources = resources.unwrap();
    assert_eq!(resources.len(), 1, "Should have 1 resource (en only)");

    // Check that only English is present
    let languages: Vec<&str> = resources
        .iter()
        .map(|r| r.metadata.language.as_str())
        .collect();
    assert!(languages.contains(&"en"), "English should be present");
    assert!(!languages.contains(&"fr"), "French should not be present");
    assert!(!languages.contains(&"es"), "Spanish should not be present");

    // Clean up
    let _ = fs::remove_file("test_include_multi.csv");
    let _ = fs::remove_file("test_include_filtered.langcodec");
}

#[test]
fn test_langcodec_include_multiple_languages() {
    // Create a test file with multiple languages
    let test_content = r#"key,en,fr,es,de
hello,Hello,Bonjour,Hola,Hallo
world,World,Monde,Mundo,Welt"#;

    fs::write("test_include_multi2.csv", test_content).expect("Failed to create test file");

    let output = langcodec_cmd()
        .args([
            "convert",
            "-i",
            "test_include_multi2.csv",
            "-o",
            "test_include_filtered2.langcodec",
            "--include-lang",
            "en",
            "--include-lang",
            "es",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_include_filtered2.langcodec").exists());

    // Validate the output structure and check that only English and Spanish are included
    let content =
        fs::read_to_string("test_include_filtered2.langcodec").expect("Failed to read output file");
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output should be valid Resource array");

    let resources = resources.unwrap();
    assert_eq!(resources.len(), 2, "Should have 2 resources (en and es)");

    // Check that only English and Spanish are present
    let languages: Vec<&str> = resources
        .iter()
        .map(|r| r.metadata.language.as_str())
        .collect();
    assert!(languages.contains(&"en"), "English should be present");
    assert!(languages.contains(&"es"), "Spanish should be present");
    assert!(!languages.contains(&"fr"), "French should not be present");
    assert!(!languages.contains(&"de"), "German should not be present");

    // Clean up
    let _ = fs::remove_file("test_include_multi2.csv");
    let _ = fs::remove_file("test_include_filtered2.langcodec");
}

#[test]
fn test_langcodec_no_include_or_exclude() {
    // Test that when no include/exclude is specified, all languages are included
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
fn test_langcodec_include_nonexistent_language() {
    // Test that including a language that doesn't exist results in empty output
    let test_content = r#"key,en,fr
hello,Hello,Bonjour
world,World,Monde"#;

    fs::write("test_include_nonexistent.csv", test_content).expect("Failed to create test file");

    let output = langcodec_cmd()
        .args([
            "convert",
            "-i",
            "test_include_nonexistent.csv",
            "-o",
            "test_include_nonexistent_filtered.langcodec",
            "--include-lang",
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
    assert!(std::path::Path::new("test_include_nonexistent_filtered.langcodec").exists());

    // Validate that no resources are present
    let content = fs::read_to_string("test_include_nonexistent_filtered.langcodec")
        .expect("Failed to read output file");
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output should be valid Resource array");

    let resources = resources.unwrap();
    assert_eq!(
        resources.len(),
        0,
        "Should have 0 resources when including non-existent language"
    );

    // Clean up
    let _ = fs::remove_file("test_include_nonexistent.csv");
    let _ = fs::remove_file("test_include_nonexistent_filtered.langcodec");
}

#[test]
fn test_langcodec_include_and_exclude_together() {
    // Test the interaction between include and exclude
    let test_content = r#"key,en,fr,es,de
hello,Hello,Bonjour,Hola,Hallo
world,World,Monde,Mundo,Welt"#;

    fs::write("test_include_exclude.csv", test_content).expect("Failed to create test file");

    let output = langcodec_cmd()
        .args([
            "convert",
            "-i",
            "test_include_exclude.csv",
            "-o",
            "test_include_exclude_filtered.langcodec",
            "--include-lang",
            "en",
            "--include-lang",
            "fr",
            "--include-lang",
            "es",
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
    assert!(std::path::Path::new("test_include_exclude_filtered.langcodec").exists());

    // Validate the output structure
    let content = fs::read_to_string("test_include_exclude_filtered.langcodec")
        .expect("Failed to read output file");
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output should be valid Resource array");

    let resources = resources.unwrap();
    assert_eq!(
        resources.len(),
        2,
        "Should have 2 resources (en and es, fr excluded)"
    );

    // Check that only English and Spanish are present (French was excluded)
    let languages: Vec<&str> = resources
        .iter()
        .map(|r| r.metadata.language.as_str())
        .collect();
    assert!(languages.contains(&"en"), "English should be present");
    assert!(languages.contains(&"es"), "Spanish should be present");
    assert!(!languages.contains(&"fr"), "French should be excluded");
    assert!(!languages.contains(&"de"), "German should not be present");

    // Clean up
    let _ = fs::remove_file("test_include_exclude.csv");
    let _ = fs::remove_file("test_include_exclude_filtered.langcodec");
}

#[test]
fn test_langcodec_include_exclude_same_language() {
    // Test edge case: include and exclude the same language
    let test_content = r#"key,en,fr,es
hello,Hello,Bonjour,Hola
world,World,Monde,Mundo"#;

    fs::write("test_include_exclude_same.csv", test_content).expect("Failed to create test file");

    let output = langcodec_cmd()
        .args([
            "convert",
            "-i",
            "test_include_exclude_same.csv",
            "-o",
            "test_include_exclude_same_filtered.langcodec",
            "--include-lang",
            "en",
            "--include-lang",
            "fr",
            "--exclude-lang",
            "fr", // Same language included and excluded
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the output file was created
    assert!(std::path::Path::new("test_include_exclude_same_filtered.langcodec").exists());

    // Validate the output structure
    let content = fs::read_to_string("test_include_exclude_same_filtered.langcodec")
        .expect("Failed to read output file");
    let resources: Result<Vec<langcodec::Resource>, _> = serde_json::from_str(&content);
    assert!(resources.is_ok(), "Output should be valid Resource array");

    let resources = resources.unwrap();
    assert_eq!(
        resources.len(),
        1,
        "Should have 1 resource (en only, fr excluded)"
    );

    // Check that only English is present (French was excluded despite being included)
    let languages: Vec<&str> = resources
        .iter()
        .map(|r| r.metadata.language.as_str())
        .collect();
    assert!(languages.contains(&"en"), "English should be present");
    assert!(!languages.contains(&"fr"), "French should be excluded");
    assert!(!languages.contains(&"es"), "Spanish should not be present");

    // Clean up
    let _ = fs::remove_file("test_include_exclude_same.csv");
    let _ = fs::remove_file("test_include_exclude_same_filtered.langcodec");
}
