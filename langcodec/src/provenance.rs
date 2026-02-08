//! Helpers for attaching and reading provenance metadata.
//!
//! Provenance is stored in `custom` maps using reserved key prefixes, so existing
//! `Resource`/`Entry` JSON shape remains backward compatible.

use serde::{Deserialize, Serialize};

use crate::types::{Entry, Resource};

pub const PROVENANCE_PREFIX: &str = "langcodec.provenance.";
const SOURCE_PATH_KEY: &str = "langcodec.provenance.source_path";
const SOURCE_FORMAT_KEY: &str = "langcodec.provenance.source_format";
const SOURCE_LANGUAGE_KEY: &str = "langcodec.provenance.source_language";
const MATCH_STRATEGY_KEY: &str = "langcodec.provenance.match_strategy";
const SOURCE_KEY_KEY: &str = "langcodec.provenance.source_key";

/// Structured provenance information for resources and entries.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub source_path: Option<String>,
    pub source_format: Option<String>,
    pub source_language: Option<String>,
    pub match_strategy: Option<String>,
    pub source_key: Option<String>,
}

impl ProvenanceRecord {
    pub fn is_empty(&self) -> bool {
        self.source_path.is_none()
            && self.source_format.is_none()
            && self.source_language.is_none()
            && self.match_strategy.is_none()
            && self.source_key.is_none()
    }
}

/// Applies provenance to a resource metadata custom map.
pub fn set_resource_provenance(resource: &mut Resource, provenance: &ProvenanceRecord) {
    apply_to_map(&mut resource.metadata.custom, provenance);
}

/// Reads provenance from a resource metadata custom map.
pub fn resource_provenance(resource: &Resource) -> Option<ProvenanceRecord> {
    from_map(&resource.metadata.custom)
}

/// Applies provenance to an entry custom map.
pub fn set_entry_provenance(entry: &mut Entry, provenance: &ProvenanceRecord) {
    apply_to_map(&mut entry.custom, provenance);
}

/// Reads provenance from an entry custom map.
pub fn entry_provenance(entry: &Entry) -> Option<ProvenanceRecord> {
    from_map(&entry.custom)
}

fn apply_to_map(
    map: &mut std::collections::HashMap<String, String>,
    provenance: &ProvenanceRecord,
) {
    apply_opt(map, SOURCE_PATH_KEY, &provenance.source_path);
    apply_opt(map, SOURCE_FORMAT_KEY, &provenance.source_format);
    apply_opt(map, SOURCE_LANGUAGE_KEY, &provenance.source_language);
    apply_opt(map, MATCH_STRATEGY_KEY, &provenance.match_strategy);
    apply_opt(map, SOURCE_KEY_KEY, &provenance.source_key);
}

fn apply_opt(
    map: &mut std::collections::HashMap<String, String>,
    key: &str,
    value: &Option<String>,
) {
    if let Some(v) = value {
        map.insert(key.to_string(), v.clone());
    } else {
        map.remove(key);
    }
}

fn from_map(map: &std::collections::HashMap<String, String>) -> Option<ProvenanceRecord> {
    let record = ProvenanceRecord {
        source_path: map.get(SOURCE_PATH_KEY).cloned(),
        source_format: map.get(SOURCE_FORMAT_KEY).cloned(),
        source_language: map.get(SOURCE_LANGUAGE_KEY).cloned(),
        match_strategy: map.get(MATCH_STRATEGY_KEY).cloned(),
        source_key: map.get(SOURCE_KEY_KEY).cloned(),
    };
    if record.is_empty() {
        None
    } else {
        Some(record)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::types::{Entry, EntryStatus, Metadata, Resource, Translation};

    use super::{
        ProvenanceRecord, entry_provenance, resource_provenance, set_entry_provenance,
        set_resource_provenance,
    };

    #[test]
    fn test_resource_provenance_roundtrip() {
        let mut resource = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "Localizable".to_string(),
                custom: HashMap::new(),
            },
            entries: Vec::new(),
        };

        let record = ProvenanceRecord {
            source_path: Some("/tmp/in.strings".to_string()),
            source_format: Some("Strings".to_string()),
            source_language: Some("en".to_string()),
            match_strategy: None,
            source_key: None,
        };

        set_resource_provenance(&mut resource, &record);
        assert_eq!(resource_provenance(&resource), Some(record));
    }

    #[test]
    fn test_entry_provenance_roundtrip() {
        let mut entry = Entry {
            id: "welcome".to_string(),
            value: Translation::Singular("Hello".to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        };

        let record = ProvenanceRecord {
            source_path: Some("/tmp/source.xcstrings".to_string()),
            source_format: Some("Xcstrings".to_string()),
            source_language: Some("fr".to_string()),
            match_strategy: Some("fallback_translation".to_string()),
            source_key: Some("welcome_title".to_string()),
        };

        set_entry_provenance(&mut entry, &record);
        assert_eq!(entry_provenance(&entry), Some(record));
    }
}
