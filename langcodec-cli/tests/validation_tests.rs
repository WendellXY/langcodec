use langcodec_cli::validation::{
    ValidationContext, validate_context, validate_custom_format, validate_file_path,
    validate_language_code, validate_output_path, validate_standard_format,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_validate_file_path_exists() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    // Create a test file
    fs::write(&test_file, "test content").unwrap();

    let result = validate_file_path(test_file.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_file_path_not_exists() {
    let result = validate_file_path("nonexistent_file.txt");
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("File does not exist"));
}

#[test]
fn test_validate_file_path_directory() {
    let temp_dir = TempDir::new().unwrap();
    let result = validate_file_path(temp_dir.path().to_str().unwrap());
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Path is not a file"));
}

#[test]
fn test_validate_output_path_writable() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    let result = validate_output_path(test_file.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_output_path_creates_directory() {
    let temp_dir = TempDir::new().unwrap();
    let nested_file = temp_dir.path().join("nested").join("test.txt");

    let result = validate_output_path(nested_file.to_str().unwrap());
    assert!(result.is_ok());

    // Verify the directory was created
    assert!(temp_dir.path().join("nested").exists());
}

#[test]
fn test_validate_language_code_valid() {
    let valid_codes = vec!["en", "fr", "es", "en-US", "fr-CA", "zh-CN", "pt-BR"];

    for code in valid_codes {
        let result = validate_language_code(code);
        assert!(result.is_ok(), "Language code '{}' should be valid", code);
    }
}

#[test]
fn test_validate_language_code_invalid() {
    let invalid_codes = vec!["", "invalid", "123", "en-", "-US"];

    for code in invalid_codes {
        let result = validate_language_code(code);
        assert!(
            result.is_err(),
            "Language code '{}' should be invalid",
            code
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("Invalid language code format") || error.contains("cannot be empty")
        );
    }
}

#[test]
fn test_validate_custom_format_valid() {
    let valid_formats = vec![
        "json-language-map",
        "yaml-language-map",
        "json-array-language-map",
        "JSON-LANGUAGE-MAP",
        "json_language_map",
    ];

    for format in valid_formats {
        let result = validate_custom_format(format);
        assert!(result.is_ok(), "Format '{}' should be valid", format);
    }
}

#[test]
fn test_validate_custom_format_invalid() {
    let invalid_formats = vec!["invalid-format", "json", "yaml", "", "json-language"];

    for format in invalid_formats {
        let result = validate_custom_format(format);
        assert!(result.is_err(), "Format '{}' should be invalid", format);
        let error = result.unwrap_err();
        assert!(error.contains("Unsupported custom format") || error.contains("cannot be empty"));
    }
}

#[test]
fn test_validate_context_complete() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.json");
    let output_file = temp_dir.path().join("output.xcstrings");

    fs::write(&input_file, "{}").unwrap();

    let context = ValidationContext::new()
        .with_input_file(input_file.to_str().unwrap().to_string())
        .with_output_file(output_file.to_str().unwrap().to_string())
        .with_language_code("en".to_string())
        .with_input_format("json-language-map".to_string())
        .with_output_format("xcstrings".to_string());

    let result = validate_context(&context);
    assert!(result.is_ok());
}

#[test]
fn test_validate_context_invalid_input_file() {
    let context = ValidationContext::new().with_input_file("nonexistent.json".to_string());

    let result = validate_context(&context);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Input file 1 validation failed"));
}

#[test]
fn test_validate_context_invalid_language() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.json");
    fs::write(&input_file, "{}").unwrap();

    let context = ValidationContext::new()
        .with_input_file(input_file.to_str().unwrap().to_string())
        .with_language_code("invalid".to_string());

    let result = validate_context(&context);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Language code validation failed"));
}

#[test]
fn test_validate_context_invalid_input_format() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.json");
    fs::write(&input_file, "{}").unwrap();

    let context = ValidationContext::new()
        .with_input_file(input_file.to_str().unwrap().to_string())
        .with_input_format("invalid-format".to_string());

    let result = validate_context(&context);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Input format validation failed"));
}

#[test]
fn test_validate_context_multiple_input_files() {
    let temp_dir = TempDir::new().unwrap();
    let input_file1 = temp_dir.path().join("input1.json");
    let input_file2 = temp_dir.path().join("input2.json");

    fs::write(&input_file1, "{}").unwrap();
    fs::write(&input_file2, "{}").unwrap();

    let context = ValidationContext::new()
        .with_input_file(input_file1.to_str().unwrap().to_string())
        .with_input_file(input_file2.to_str().unwrap().to_string());

    let result = validate_context(&context);
    assert!(result.is_ok());
}

#[test]
fn test_validate_context_empty() {
    let context = ValidationContext::new();
    let result = validate_context(&context);
    assert!(result.is_ok());
}

#[test]
fn test_validation_context_builder() {
    let context = ValidationContext::new()
        .with_input_file("test.json".to_string())
        .with_output_file("output.xcstrings".to_string())
        .with_language_code("en".to_string())
        .with_input_format("json-language-map".to_string())
        .with_output_format("xcstrings".to_string());

    assert_eq!(context.input_files.len(), 1);
    assert_eq!(context.input_files[0], "test.json");
    assert_eq!(context.output_file.as_ref().unwrap(), "output.xcstrings");
    assert_eq!(context.language_code.as_ref().unwrap(), "en");
    assert_eq!(context.input_format.as_ref().unwrap(), "json-language-map");
    assert_eq!(context.output_format.as_ref().unwrap(), "xcstrings");
}

#[test]
fn test_validate_file_permissions() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    // Create a file
    fs::write(&test_file, "test content").unwrap();

    // Test that we can read the file
    let result = validate_file_path(test_file.to_str().unwrap());
    assert!(result.is_ok());

    // Test that we can write to output location
    let output_file = temp_dir.path().join("output.txt");
    let result = validate_output_path(output_file.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_special_characters_in_path() {
    let temp_dir = TempDir::new().unwrap();

    // Test with spaces in filename
    let file_with_spaces = temp_dir.path().join("test file.txt");
    fs::write(&file_with_spaces, "test content").unwrap();

    let result = validate_file_path(file_with_spaces.to_str().unwrap());
    assert!(result.is_ok());

    // Test with special characters in filename
    let file_with_special_chars = temp_dir.path().join("test-file_123.txt");
    fs::write(&file_with_special_chars, "test content").unwrap();

    let result = validate_file_path(file_with_special_chars.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_unicode_paths() {
    let temp_dir = TempDir::new().unwrap();

    // Test with unicode characters in filename
    let unicode_file = temp_dir.path().join("test-émojis-🚀.txt");
    fs::write(&unicode_file, "test content").unwrap();

    let result = validate_file_path(unicode_file.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_relative_paths() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Change to temp directory and test relative path
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let result = validate_file_path("test.txt");
    assert!(result.is_ok());

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_validate_absolute_paths() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    let result = validate_file_path(test_file.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_case_sensitivity() {
    // Test case sensitivity in format names
    let result1 = validate_custom_format("JSON-LANGUAGE-MAP");
    let result2 = validate_custom_format("json-language-map");

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[test]
fn test_validate_whitespace_handling() {
    // Test that whitespace is handled properly in format validation
    let result1 = validate_custom_format("  json-language-map  ");
    let result2 = validate_standard_format("  xcstrings  ");
    let result3 = validate_standard_format("  tsv  ");

    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(result3.is_ok());
}

#[test]
fn test_validate_error_messages() {
    // Test that error messages are descriptive
    let result = validate_file_path("nonexistent.txt");
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("File does not exist"));
    assert!(error.contains("nonexistent.txt"));

    let result = validate_custom_format("invalid");
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Unsupported custom format"));
    assert!(error.contains("invalid"));
}

#[test]
fn test_validate_file_size_limits() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("large_file.txt");

    // Create a large file (1MB)
    let large_content = "x".repeat(1024 * 1024);
    fs::write(&test_file, large_content).unwrap();

    let result = validate_file_path(test_file.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_symlinks() {
    let temp_dir = TempDir::new().unwrap();
    let original_file = temp_dir.path().join("original.txt");
    let symlink_file = temp_dir.path().join("symlink.txt");

    fs::write(&original_file, "test content").unwrap();

    // Create symlink (this might not work on all platforms)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        if symlink(&original_file, &symlink_file).is_ok() {
            let result = validate_file_path(symlink_file.to_str().unwrap());
            assert!(result.is_ok());
        }
    }
}

#[test]
fn test_validate_concurrent_access() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    fs::write(&test_file, "test content").unwrap();

    // Test that multiple validations of the same file work
    let result1 = validate_file_path(test_file.to_str().unwrap());
    let result2 = validate_file_path(test_file.to_str().unwrap());

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}
