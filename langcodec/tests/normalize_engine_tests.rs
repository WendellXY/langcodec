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
fn normalize_rejects_placeholder_option_until_implemented() {
    let mut codec = Codec { resources: vec![] };
    let options = NormalizeOptions {
        normalize_placeholders: true,
        key_style: KeyStyle::None,
    };

    let error = langcodec::normalize::normalize_codec(&mut codec, &options).unwrap_err();
    assert!(error.to_string().contains("not yet implemented"));
}

#[test]
fn normalize_rejects_key_style_option_until_implemented() {
    let mut codec = Codec { resources: vec![] };
    let options = NormalizeOptions {
        normalize_placeholders: false,
        key_style: KeyStyle::Snake,
    };

    let error = langcodec::normalize::normalize_codec(&mut codec, &options).unwrap_err();
    assert!(error.to_string().contains("not yet implemented"));
}
