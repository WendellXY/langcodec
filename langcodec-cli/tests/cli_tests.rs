use langcodec_cli::{CustomFormat, custom_format_to_resource, parse_custom_format};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_parse_custom_format_json() {
    let result = parse_custom_format("json-language-map");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), CustomFormat::JSONLanguageMap));
}

#[test]
fn test_parse_custom_format_yaml() {
    let result = parse_custom_format("yaml-language-map");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), CustomFormat::YAMLLanguageMap));
}

#[test]
fn test_parse_custom_format_case_insensitive() {
    let result = parse_custom_format("JSON-LANGUAGE-MAP");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), CustomFormat::JSONLanguageMap));
}

#[test]
fn test_parse_custom_format_with_underscores() {
    let result = parse_custom_format("json_language_map");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), CustomFormat::JSONLanguageMap));
}

#[test]
fn test_parse_custom_format_invalid() {
    let result = parse_custom_format("invalid-format");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown custom format"));
}

#[test]
fn test_json_language_map_transformation() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("test.json");

    let json_content = r#"{
        "key": "hello_world",
        "en": "Hello, World!",
        "fr": "Bonjour, le monde!"
    }"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );
    assert!(result.is_ok());

    let resources = result.unwrap();
    assert_eq!(resources.len(), 2); // en and fr, excluding "key"

    // Check English resource
    let en_resource = resources
        .iter()
        .find(|r| r.metadata.language == "en")
        .unwrap();
    assert_eq!(en_resource.entries.len(), 1);
    assert_eq!(en_resource.entries[0].id, "hello_world");
    assert_eq!(
        en_resource.entries[0].value.plain_translation_string(),
        "Hello, World!"
    );

    // Check French resource
    let fr_resource = resources
        .iter()
        .find(|r| r.metadata.language == "fr")
        .unwrap();
    assert_eq!(fr_resource.entries.len(), 1);
    assert_eq!(fr_resource.entries[0].id, "hello_world");
    assert_eq!(
        fr_resource.entries[0].value.plain_translation_string(),
        "Bonjour, le monde!"
    );
}

#[test]
fn test_yaml_language_map_transformation() {
    let temp_dir = TempDir::new().unwrap();
    let yaml_file = temp_dir.path().join("test.yaml");

    let yaml_content = r#"key: hello_world
en: Hello, World!
fr: Bonjour, le monde!"#;

    fs::write(&yaml_file, yaml_content).unwrap();

    let result = custom_format_to_resource(
        yaml_file.to_string_lossy().to_string(),
        CustomFormat::YAMLLanguageMap,
    );
    assert!(result.is_ok());

    let resources = result.unwrap();
    assert_eq!(resources.len(), 2); // en and fr, excluding "key"

    // Check English resource
    let en_resource = resources
        .iter()
        .find(|r| r.metadata.language == "en")
        .unwrap();
    assert_eq!(en_resource.entries.len(), 1);
    assert_eq!(en_resource.entries[0].id, "hello_world");
    assert_eq!(
        en_resource.entries[0].value.plain_translation_string(),
        "Hello, World!"
    );

    // Check French resource
    let fr_resource = resources
        .iter()
        .find(|r| r.metadata.language == "fr")
        .unwrap();
    assert_eq!(fr_resource.entries.len(), 1);
    assert_eq!(fr_resource.entries[0].id, "hello_world");
    assert_eq!(
        fr_resource.entries[0].value.plain_translation_string(),
        "Bonjour, le monde!"
    );
}

#[test]
fn test_json_transformation_without_key_field() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("test.json");

    let json_content = r#"{
        "en": "Hello, World!",
        "fr": "Bonjour, le monde!"
    }"#;

    fs::write(&json_file, json_content).unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );
    assert!(result.is_ok());

    let resources = result.unwrap();
    assert_eq!(resources.len(), 2);

    // Should use "en" value as the key since no "key" field is present
    let en_resource = resources
        .iter()
        .find(|r| r.metadata.language == "en")
        .unwrap();
    assert_eq!(en_resource.entries[0].id, "Hello, World!");
}

#[test]
fn test_json_transformation_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("empty.json");

    fs::write(&json_file, "{}").unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("JSON object is empty"));
}

#[test]
fn test_json_transformation_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("invalid.json");

    fs::write(&json_file, "{ invalid json }").unwrap();

    let result = custom_format_to_resource(
        json_file.to_string_lossy().to_string(),
        CustomFormat::JSONLanguageMap,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Error parsing JSON"));
}

#[test]
fn test_yaml_transformation_invalid_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let yaml_file = temp_dir.path().join("invalid.yaml");

    fs::write(&yaml_file, "invalid: yaml: content").unwrap();

    let result = custom_format_to_resource(
        yaml_file.to_string_lossy().to_string(),
        CustomFormat::YAMLLanguageMap,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Error parsing YAML"));
}

#[test]
fn test_nonexistent_file() {
    let result = custom_format_to_resource(
        "nonexistent.json".to_string(),
        CustomFormat::JSONLanguageMap,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Error reading file"));
}
