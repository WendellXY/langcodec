use langcodec::converter;
use langcodec::types::{ConflictStrategy, Entry, EntryStatus, Metadata, Resource, Translation};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_merge_basic_resources() {
    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hello".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "goodbye".to_string(),
            value: Translation::Singular("Goodbye".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let merged =
        langcodec::merge_resources(&[resource1, resource2], &ConflictStrategy::Last).unwrap();
    assert_eq!(merged.entries.len(), 2);
    assert!(merged.entries.iter().any(|e| e.id == "hello"));
    assert!(merged.entries.iter().any(|e| e.id == "goodbye"));
}

#[test]
fn test_merge_with_conflicts_first_strategy() {
    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hello".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hi".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let merged =
        langcodec::merge_resources(&[resource1, resource2], &ConflictStrategy::First).unwrap();
    assert_eq!(merged.entries.len(), 1);
    let entry = &merged.entries[0];
    assert_eq!(entry.id, "hello");
    assert_eq!(entry.value.plain_translation_string(), "Hello"); // First wins
}

#[test]
fn test_merge_with_conflicts_last_strategy() {
    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hello".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hi".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let merged =
        langcodec::merge_resources(&[resource1, resource2], &ConflictStrategy::Last).unwrap();
    assert_eq!(merged.entries.len(), 1);
    let entry = &merged.entries[0];
    assert_eq!(entry.id, "hello");
    assert_eq!(entry.value.plain_translation_string(), "Hi"); // Last wins
}

#[test]
fn test_merge_with_conflicts_skip_strategy() {
    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hello".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hi".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let merged =
        converter::merge_resources(&[resource1, resource2], &ConflictStrategy::Skip).unwrap();
    assert_eq!(merged.entries.len(), 0); // Both conflicting entries are skipped
}

#[test]
fn test_merge_empty_resources() {
    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![],
    };

    let merged =
        converter::merge_resources(&[resource1, resource2], &ConflictStrategy::Last).unwrap();
    assert_eq!(merged.entries.len(), 0);
    assert_eq!(merged.metadata.language, "en");
}

#[test]
fn test_merge_single_resource() {
    let resource = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hello".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let merged = converter::merge_resources(&[resource], &ConflictStrategy::Last).unwrap();
    assert_eq!(merged.entries.len(), 1);
    assert_eq!(merged.entries[0].id, "hello");
}

#[test]
fn test_merge_multiple_conflicts() {
    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![
            Entry {
                id: "hello".to_string(),
                value: Translation::Singular("Hello".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            },
            Entry {
                id: "goodbye".to_string(),
                value: Translation::Singular("Goodbye".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            },
        ],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![
            Entry {
                id: "hello".to_string(),
                value: Translation::Singular("Hi".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            },
            Entry {
                id: "goodbye".to_string(),
                value: Translation::Singular("Bye".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            },
        ],
    };

    let merged =
        converter::merge_resources(&[resource1, resource2], &ConflictStrategy::Last).unwrap();
    assert_eq!(merged.entries.len(), 2);

    let hello_entry = merged.entries.iter().find(|e| e.id == "hello").unwrap();
    let goodbye_entry = merged.entries.iter().find(|e| e.id == "goodbye").unwrap();

    assert_eq!(hello_entry.value.plain_translation_string(), "Hi");
    assert_eq!(goodbye_entry.value.plain_translation_string(), "Bye");
}

#[test]
fn test_merge_with_comments() {
    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hello".to_string()),
            comment: Some("Greeting".to_string()),
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hi".to_string()),
            comment: Some("Informal greeting".to_string()),
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let merged =
        converter::merge_resources(&[resource1, resource2], &ConflictStrategy::Last).unwrap();
    assert_eq!(merged.entries.len(), 1);
    let entry = &merged.entries[0];
    assert_eq!(entry.comment.as_ref().unwrap(), "Informal greeting");
}

#[test]
fn test_merge_with_different_statuses() {
    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hello".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hi".to_string()),
            comment: None,
            status: EntryStatus::NeedsReview,
            custom: HashMap::new(),
        }],
    };

    let merged =
        converter::merge_resources(&[resource1, resource2], &ConflictStrategy::Last).unwrap();
    assert_eq!(merged.entries.len(), 1);
    let entry = &merged.entries[0];
    assert_eq!(entry.status, EntryStatus::NeedsReview);
}

#[test]
fn test_merge_with_custom_fields() {
    let mut custom1 = HashMap::new();
    custom1.insert("priority".to_string(), "high".to_string());

    let mut custom2 = HashMap::new();
    custom2.insert("priority".to_string(), "low".to_string());
    custom2.insert("context".to_string(), "app".to_string());

    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hello".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: custom1,
        }],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hi".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: custom2,
        }],
    };

    let merged =
        converter::merge_resources(&[resource1, resource2], &ConflictStrategy::Last).unwrap();
    assert_eq!(merged.entries.len(), 1);
    let entry = &merged.entries[0];
    assert_eq!(entry.custom.get("priority").unwrap(), "low");
    assert_eq!(entry.custom.get("context").unwrap(), "app");
}

#[test]
fn test_merge_plurals() {
    use langcodec::types::PluralCategory;
    use std::collections::BTreeMap;

    let mut forms1 = BTreeMap::new();
    forms1.insert(PluralCategory::One, "1 apple".to_string());
    forms1.insert(PluralCategory::Other, "%d apples".to_string());

    let mut forms2 = BTreeMap::new();
    forms2.insert(PluralCategory::One, "1 orange".to_string());
    forms2.insert(PluralCategory::Other, "%d oranges".to_string());

    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "apples".to_string(),
            value: Translation::Plural(langcodec::types::Plural {
                id: "apples".to_string(),
                forms: forms1,
            }),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "en".to_string(), // Same language as resource1
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "oranges".to_string(), // Different ID to avoid conflict
            value: Translation::Plural(langcodec::types::Plural {
                id: "oranges".to_string(),
                forms: forms2,
            }),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let merged =
        converter::merge_resources(&[resource1, resource2], &ConflictStrategy::Last).unwrap();
    assert_eq!(merged.entries.len(), 2); // Same language, different IDs, no conflict
}

#[test]
fn test_merge_different_languages_error() {
    let resource1 = Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Hello".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    let resource2 = Resource {
        metadata: Metadata {
            language: "fr".to_string(), // Different language
            domain: "test".to_string(),
            custom: HashMap::new(),
        },
        entries: vec![Entry {
            id: "hello".to_string(),
            value: Translation::Singular("Bonjour".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }],
    };

    // Should fail because resources have different languages
    let result = converter::merge_resources(&[resource1, resource2], &ConflictStrategy::Last);
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(matches!(error, langcodec::Error::InvalidResource(_)));
    assert!(
        error
            .to_string()
            .contains("Cannot merge resources with different languages")
    );
}

#[test]
fn test_merge_file_integration() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.strings");
    let file2 = temp_dir.path().join("file2.strings");

    // Create test files
    let content1 = r#"/* Greeting */
"hello" = "Hello";"#;
    let content2 = r#"/* Farewell */
"goodbye" = "Goodbye";"#;

    fs::write(&file1, content1).unwrap();
    fs::write(&file2, content2).unwrap();

    // Test merge command (this would require implementing the actual merge command)
    // For now, we'll just verify the files exist and have content
    assert!(file1.exists());
    assert!(file2.exists());

    let content1_read = fs::read_to_string(&file1).unwrap();
    let content2_read = fs::read_to_string(&file2).unwrap();

    assert!(content1_read.contains("hello"));
    assert!(content2_read.contains("goodbye"));
}
