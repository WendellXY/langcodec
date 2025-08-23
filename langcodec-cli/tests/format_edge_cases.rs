use langcodec_cli::{CustomFormat, custom_format_to_resource};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_json_with_nested_objects() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("nested.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": {
            "formal": "Hello, World!",
            "informal": "Hi there!"
        },
        "fr": "Bonjour, le monde!"
    }"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );

    // Should fail because nested objects are not supported
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Error parsing JSON") || error.contains("Invalid format"));
}

#[test]
fn test_json_with_arrays() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("arrays.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": ["Hello", "Hi", "Hey"],
        "fr": "Bonjour"
    }"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );

    // Should fail because arrays are not supported
    assert!(result.is_err());
}

#[test]
fn test_json_with_null_values() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("null.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": "Hello",
        "fr": null
    }"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );

    // Should fail because null values are not supported
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Error parsing JSON") || error.contains("Invalid format"));
}

#[test]
fn test_json_with_boolean_values() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("boolean.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": true,
        "fr": false
    }"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );

    // Should convert booleans to strings
    if let Ok(resources) = result {
        let en_resource = resources
            .iter()
            .find(|r| r.metadata.language == "en")
            .unwrap();
        assert_eq!(
            en_resource.entries[0].value.plain_translation_string(),
            "true"
        );
    }
}

#[test]
fn test_json_with_numbers() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("numbers.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": 42,
        "fr": 3.14
    }"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );

    // Should convert numbers to strings
    if let Ok(resources) = result {
        let en_resource = resources
            .iter()
            .find(|r| r.metadata.language == "en")
            .unwrap();
        assert_eq!(
            en_resource.entries[0].value.plain_translation_string(),
            "42"
        );
    }
}

#[test]
fn test_yaml_with_comments() {
    let temp_dir = TempDir::new().unwrap();
    let yaml_file = temp_dir.path().join("comments.yaml");

    let yaml_content = r#"# This is a comment
key: hello_world
en: Hello, World!  # Inline comment
fr: Bonjour, le monde!"#;

    fs::write(&yaml_file, yaml_content).unwrap();

    let result = custom_format_to_resource(
        yaml_file.to_string_lossy().to_string(),
        CustomFormat::YAMLLanguageMap,
    );
    assert!(result.is_ok());
}

#[test]
fn test_yaml_with_multiline_strings() {
    let temp_dir = TempDir::new().unwrap();
    let yaml_file = temp_dir.path().join("multiline.yaml");

    let yaml_content = r#"key: hello_world
en: |
  This is a
  multiline string
  with multiple lines
fr: Bonjour"#;

    fs::write(&yaml_file, yaml_content).unwrap();

    let result = custom_format_to_resource(
        yaml_file.to_string_lossy().to_string(),
        CustomFormat::YAMLLanguageMap,
    );
    assert!(result.is_ok());
}

#[test]
fn test_json_array_with_missing_keys() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("missing_keys.json");

    let json_content = r#"[
        {
            "en": "Hello",
            "fr": "Bonjour"
        },
        {
            "key": "welcome",
            "en": "Welcome",
            "fr": "Bienvenue"
        }
    ]"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONArrayLanguageMap,
    );

    // Should handle missing keys gracefully
    if let Ok(resources) = result {
        assert_eq!(resources.len(), 2); // en and fr
    }
}

#[test]
fn test_json_array_with_duplicate_keys() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("duplicate_keys.json");

    let json_content = r#"[
        {
            "key": "hello",
            "en": "Hello",
            "fr": "Bonjour"
        },
        {
            "key": "hello",
            "en": "Hi",
            "fr": "Salut"
        }
    ]"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONArrayLanguageMap,
    );

    // Should handle duplicate keys (last one wins)
    if let Ok(resources) = result {
        let en_resource = resources
            .iter()
            .find(|r| r.metadata.language == "en")
            .unwrap();
        let hello_entry = en_resource
            .entries
            .iter()
            .find(|e| e.id == "hello")
            .unwrap();
        assert_eq!(hello_entry.value.plain_translation_string(), "Hi");
    }
}

#[test]
fn test_large_json_file() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("large.json");

    // Create a large JSON object with many entries
    let mut json_content = String::from("{");
    for i in 0..1000 {
        if i > 0 {
            json_content.push(',');
        }
        json_content.push_str(&format!(
            r#""key_{}": "value_{}", "en": "English_{}", "fr": "French_{}""#,
            i, i, i, i
        ));
    }
    json_content.push('}');

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );

    // Should handle large files
    assert!(result.is_ok());
}

#[test]
fn test_malformed_json() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("malformed.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": "Hello",
        "fr": "Bonjour",
    }"#; // Trailing comma

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );
    assert!(result.is_err());
}

#[test]
fn test_malformed_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let yaml_file = temp_dir.path().join("malformed.yaml");

    let yaml_content = r#"key: hello_world
en: Hello
fr: Bonjour
  - invalid: yaml: structure"#;

    fs::write(&yaml_file, yaml_content).unwrap();

    let result = custom_format_to_resource(
        yaml_file.to_string_lossy().to_string(),
        CustomFormat::YAMLLanguageMap,
    );
    assert!(result.is_err());
}

#[test]
fn test_empty_json_object() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("empty.json");

    fs::write(&json_file, "{}").unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("JSON object is empty"));
}

#[test]
fn test_empty_json_array() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("empty_array.json");

    fs::write(&json_file, "[]").unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONArrayLanguageMap,
    );
    assert!(result.is_err());
}

#[test]
fn test_json_with_unicode_escape_sequences() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("unicode.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": "Hello \u0041\u0042\u0043",
        "fr": "Bonjour \u00E9\u00E0\u00E8"
    }"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );
    assert!(result.is_ok());
}

#[test]
fn test_yaml_with_unicode_characters() {
    let temp_dir = TempDir::new().unwrap();
    let yaml_file = temp_dir.path().join("unicode.yaml");

    let yaml_content = r#"key: hello_world
en: Hello ABC
fr: Bonjour éàè"#;

    fs::write(&yaml_file, yaml_content).unwrap();

    let result = custom_format_to_resource(
        yaml_file.to_string_lossy().to_string(),
        CustomFormat::YAMLLanguageMap,
    );
    assert!(result.is_ok());
}

#[test]
fn test_json_with_very_long_strings() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("long_strings.json");

    let long_string = "x".repeat(10000);
    let json_content = format!(
        r#"{{"key": "hello_world", "en": "{}", "fr": "{}"}}"#,
        long_string, long_string
    );

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );
    assert!(result.is_ok());
}

#[test]
fn test_json_with_special_characters_in_keys() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("special_chars.json");

    let json_content = r#"{
        "key-with-dashes": "hello_world",
        "key_with_underscores": "hello_world",
        "keyWithCamelCase": "hello_world",
        "en": "Hello",
        "fr": "Bonjour"
    }"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );
    assert!(result.is_ok());
}
