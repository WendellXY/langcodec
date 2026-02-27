use langcodec::{
    Codec,
    normalize::{KeyStyle, NormalizeOptions},
    types::{Entry, EntryStatus, Metadata, Resource, Translation},
};
use std::collections::HashMap;

#[test]
fn normalize_sorts_entries_and_is_idempotent() {
    let mut codec = Codec {
        resources: vec![Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "Localizable".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![
                Entry {
                    id: "z_key".to_string(),
                    value: Translation::Singular("Z".to_string()),
                    comment: None,
                    status: EntryStatus::Translated,
                    custom: HashMap::new(),
                },
                Entry {
                    id: "a_key".to_string(),
                    value: Translation::Singular("A".to_string()),
                    comment: None,
                    status: EntryStatus::Translated,
                    custom: HashMap::new(),
                },
            ],
        }],
    };

    let report1 = langcodec::normalize::normalize_codec(&mut codec, &Default::default()).unwrap();
    let ids: Vec<_> = codec.resources[0]
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(ids, vec!["a_key", "z_key"]);
    assert!(report1.changed);

    let report2 = langcodec::normalize::normalize_codec(&mut codec, &Default::default()).unwrap();
    assert!(!report2.changed);
}

#[test]
fn normalize_applies_placeholder_normalization_by_default() {
    let mut codec = Codec {
        resources: vec![Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "Localizable".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "summary".to_string(),
                value: Translation::Singular("%@ has %ld items".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        }],
    };

    let report = langcodec::normalize::normalize_codec(&mut codec, &Default::default()).unwrap();
    let value = match &codec.resources[0].entries[0].value {
        Translation::Singular(value) => value.clone(),
        _ => unreachable!("test fixture uses singular translation"),
    };

    assert_eq!(value, "%s has %d items");
    assert!(report.changed);
}

#[test]
fn normalize_errors_on_key_style_collision_after_transform() {
    let mut codec = Codec {
        resources: vec![Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "Localizable".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![
                Entry {
                    id: "welcome-title".to_string(),
                    value: Translation::Singular("Welcome".to_string()),
                    comment: None,
                    status: EntryStatus::Translated,
                    custom: HashMap::new(),
                },
                Entry {
                    id: "welcome_title".to_string(),
                    value: Translation::Singular("Welcome again".to_string()),
                    comment: None,
                    status: EntryStatus::Translated,
                    custom: HashMap::new(),
                },
            ],
        }],
    };

    let options = NormalizeOptions {
        normalize_placeholders: false,
        key_style: KeyStyle::Snake,
    };

    let error = langcodec::normalize::normalize_codec(&mut codec, &options).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("collision"));
    assert!(message.contains("welcome-title"));
    assert!(message.contains("welcome_title"));
}
