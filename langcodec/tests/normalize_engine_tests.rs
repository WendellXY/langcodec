use langcodec::{
    Codec,
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
