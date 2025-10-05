use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{BufRead, Write},
    str::FromStr,
};

use crate::{
    error::Error,
    traits::Parser,
    types::{Entry, EntryStatus, Metadata, Plural, PluralCategory, Resource, Translation},
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub source_language: String,
    pub version: String,
    pub strings: HashMap<String, Item>,
}

impl Parser for Format {
    /// Parses the xcstrings format from a reader.
    fn from_reader<R: BufRead>(reader: R) -> Result<Self, Error> {
        serde_json::from_reader(reader).map_err(Error::Parse)
    }

    /// Serializes the xcstrings format to a writer.
    fn to_writer<W: Write>(&self, writer: W) -> Result<(), Error> {
        serde_json::to_writer_pretty(writer, &self).map_err(Error::Parse)
    }
}

impl TryFrom<Vec<Resource>> for Format {
    type Error = Error;

    fn try_from(resources: Vec<Resource>) -> Result<Self, Self::Error> {
        // Key: String ID of the item (e.g. "hello world")
        // Value: Item containing localizations and metadata
        let mut strings = HashMap::<String, Item>::new();
        let mut source_language = String::new();
        let mut version = String::new();

        for mut resource in resources {
            // source_language
            if source_language.is_empty() {
                if let Some(v) = resource.metadata.custom.remove("source_language") {
                    source_language = v; // moved, no clone
                } else {
                    return Err(Error::InvalidResource(
                        "No source language found in metadata".into(),
                    ));
                }
            } else if let Some(v) = resource.metadata.custom.get("source_language") {
                if source_language != *v {
                    return Err(Error::DataMismatch(format!(
                        "Source language mismatch: expected {}, found {}",
                        source_language, v
                    )));
                }
            } else {
                return Err(Error::InvalidResource(
                    "No source language found in metadata".into(),
                ));
            }

            if version.is_empty() {
                if let Some(v) = resource.metadata.custom.remove("version") {
                    version = v; // move, no clone
                } else {
                    return Err(Error::InvalidResource(
                        "No version found in metadata".into(),
                    ));
                }
            } else if let Some(v) = resource.metadata.custom.get("version") {
                if version != *v {
                    return Err(Error::DataMismatch(format!(
                        "Version mismatch: expected {}, found {}",
                        version, v
                    )));
                }
            } else {
                return Err(Error::InvalidResource(
                    "No version found in metadata".into(),
                ));
            }

            for entry in resource.entries {
                let id = entry.id.clone();
                if let Some(item) = Item::new(entry, resource.metadata.language.clone()) {
                    strings
                        .entry(id)
                        .or_insert(Item {
                            localizations: HashMap::new(),
                            comment: item.comment,
                            extraction_state: item.extraction_state,
                            should_translate: item.should_translate,
                            is_comment_auto_generated: item.is_comment_auto_generated,
                        })
                        .localizations
                        .extend(item.localizations);
                }
            }
        }

        Ok(Format {
            source_language,
            version,
            strings,
        })
    }
}

impl TryFrom<Format> for Vec<Resource> {
    type Error = Error;

    fn try_from(format: Format) -> Result<Self, Self::Error> {
        // Key: Language code, e.g. "en", "fr", etc.
        // Value: Resource containing all items for that language
        let mut resource_map = HashMap::<String, Resource>::new();

        let mut custom_meta = HashMap::<String, String>::new();
        custom_meta.insert(String::from("source_language"), format.source_language);
        custom_meta.insert(String::from("version"), format.version);

        for (id, item) in format.strings {
            let mut custom = HashMap::new();
            if let Some(extraction_state) = &item.extraction_state {
                custom.insert("extraction_state".to_string(), extraction_state.to_string());
            }

            if item.localizations.is_empty() {
                if item.should_translate.unwrap_or(true) {
                    // If the item is empty and should be translated, add a new entry for each language
                    //
                    // This method requires that all languages are already present in the resource map, in
                    // other words, the translated items must be presented above the untranslated items.
                    let lang_codes = resource_map.keys().cloned().collect::<Vec<_>>();
                    for lang_code in lang_codes {
                        resource_map
                            .entry(lang_code.clone())
                            .or_insert(Resource {
                                metadata: Metadata {
                                    language: lang_code.clone(),
                                    domain: String::default(),
                                    custom: custom_meta.clone(),
                                },
                                entries: Vec::new(),
                            })
                            .add_entry(Entry {
                                id: id.clone(),
                                value: Translation::Singular("".to_string()),
                                comment: item.comment.clone(),
                                status: EntryStatus::Translated,
                                custom: custom.clone(),
                            });
                    }
                }
                continue;
            }

            for (lang_code, localization) in item.localizations {
                if let Some(translation) = localization.to_translation() {
                    let lang_code = lang_code.to_string();
                    resource_map
                        .entry(lang_code.clone())
                        .or_insert(Resource {
                            metadata: Metadata {
                                language: lang_code.clone(),
                                domain: String::default(),
                                custom: custom_meta.clone(),
                            },
                            entries: Vec::new(),
                        })
                        .add_entry(Entry {
                            id: id.clone(),
                            value: translation,
                            comment: item.comment.clone(),
                            status: localization.state(),
                            custom: custom.clone(), // No custom data in xcstrings
                        });
                }
            }
        }

        Ok(resource_map.into_values().collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub localizations: HashMap<String, Localization>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_state: Option<ExtractionState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should_translate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_comment_auto_generated: Option<bool>,
}

impl Item {
    fn new(entry: Entry, language: String) -> Option<Self> {
        let mut localizations = HashMap::new();

        let should_translate = Some(entry.status != EntryStatus::DoNotTranslate);

        match entry.value {
            Translation::Empty => {} // Do nothing
            Translation::Singular(value) => {
                localizations.insert(
                    language,
                    Localization::from(StringUnit::new(entry.status, &value)),
                );
            }
            Translation::Plural(plural) => {
                localizations.insert(
                    language,
                    Localization::from(Variations::new(plural.forms.iter().map(
                        |(category, value)| {
                            (
                                category.clone(),
                                PluralVariation::new(entry.status.clone(), value),
                            )
                        },
                    ))),
                );
            }
        }

        let extraction_state = entry
            .custom
            .get("extraction_state")
            .and_then(|s| s.parse::<ExtractionState>().ok());

        let is_comment_auto_generated = entry
            .custom
            .get("is_comment_auto_generated")
            .and_then(|s| s.parse::<bool>().ok());

        Some(Item {
            localizations,
            comment: entry.comment,
            extraction_state,
            should_translate,
            is_comment_auto_generated,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionState {
    Manual,
    Stale,
    ExtractedWithValue,
    Migrated,
}

impl std::fmt::Display for ExtractionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractionState::Manual => write!(f, "manual"),
            ExtractionState::Stale => write!(f, "stale"),
            ExtractionState::ExtractedWithValue => write!(f, "extracted_with_value"),
            ExtractionState::Migrated => write!(f, "migrated"),
        }
    }
}

impl FromStr for ExtractionState {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "manual" => Ok(ExtractionState::Manual),
            "stale" => Ok(ExtractionState::Stale),
            "extracted_with_value" => Ok(ExtractionState::ExtractedWithValue),
            "migrated" => Ok(ExtractionState::Migrated),
            _ => Err(Error::DataMismatch(format!(
                "Unknown extraction state: {}",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Localization {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_unit: Option<StringUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variations: Option<Variations>,
}

impl From<StringUnit> for Localization {
    fn from(string_unit: StringUnit) -> Self {
        Localization {
            string_unit: Some(string_unit),
            variations: None,
        }
    }
}

impl From<Variations> for Localization {
    fn from(variations: Variations) -> Self {
        Localization {
            string_unit: None,
            variations: Some(variations),
        }
    }
}

impl Localization {
    fn to_translation(&self) -> Option<Translation> {
        match (self.string_unit.as_ref(), self.variations.as_ref()) {
            (Some(string_unit), _) => Some(Translation::Singular(string_unit.value.clone())),
            (_, Some(variations)) => variations.to_translation(),
            (None, None) => None,
        }
    }

    fn state(&self) -> EntryStatus {
        if let Some(string_unit) = &self.string_unit {
            string_unit.state.clone()
        } else if let Some(variations) = &self.variations {
            // If variations exist, we assume all variations are in the same state
            variations
                .plural
                .as_ref()
                .and_then(|plural_map| {
                    plural_map.values().next().and_then(|variation| {
                        variation.string_unit.as_ref().map(|su| su.state.clone())
                    })
                })
                .unwrap_or(EntryStatus::Stale)
        } else {
            EntryStatus::Stale
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct StringUnit {
    pub state: EntryStatus,
    pub value: String,
}

impl StringUnit {
    pub fn new(state: EntryStatus, value: &str) -> Self {
        Self {
            state,
            value: crate::placeholder::to_ios_placeholders(value).replace("\\n", "\n"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Variations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plural: Option<HashMap<PluralCategory, PluralVariation>>,
}

impl Variations {
    pub fn new(plural: impl Iterator<Item = (PluralCategory, PluralVariation)>) -> Self {
        let plural = plural.collect();
        Self {
            plural: Some(plural),
        }
    }
}

impl Variations {
    fn to_translation(&self) -> Option<Translation> {
        self.plural.as_ref().and_then(|plural_map| {
            let forms = plural_map.iter().filter_map(|(category, variation)| {
                let category = category.clone();
                let value = variation.string_unit.as_ref()?.value.clone();
                Some((category, value))
            });

            Plural::new("", forms).map(Translation::Plural)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluralVariation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_unit: Option<StringUnit>,
}

impl PluralVariation {
    pub fn new(state: EntryStatus, value: &str) -> Self {
        Self {
            string_unit: Some(StringUnit::new(state, value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ios_placeholder_conversion_in_writer() {
        // Build resources that contain Android-style placeholders
        let res = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: String::new(),
                custom: {
                    let mut m = HashMap::new();
                    m.insert("source_language".into(), "en".into());
                    m.insert("version".into(), "1.0".into());
                    m
                },
            },
            entries: vec![
                Entry {
                    id: "greet".into(),
                    value: Translation::Singular("Hello %1$s and %s".into()),
                    comment: None,
                    status: EntryStatus::Translated,
                    custom: HashMap::new(),
                },
                Entry {
                    id: "files".into(),
                    value: Translation::Plural(Plural {
                        id: "files".into(),
                        forms: {
                            let mut f = std::collections::BTreeMap::new();
                            f.insert(PluralCategory::One, "%1$s file".into());
                            f.insert(PluralCategory::Other, "%1$s files".into());
                            f
                        },
                    }),
                    comment: None,
                    status: EntryStatus::Translated,
                    custom: HashMap::new(),
                },
            ],
        };

        let fmt = Format::try_from(vec![res]).expect("xcstrings from resources");
        // greet
        let item = fmt.strings.get("greet").expect("greet item");
        let en = item.localizations.get("en").expect("en loc");
        let val = en.string_unit.as_ref().unwrap().value.clone();
        assert!(val.contains("%1$@") && val.contains("%@"));

        // plurals
        let files = fmt.strings.get("files").expect("files item");
        let en_p = files.localizations.get("en").expect("en loc");
        let plural_map = en_p.variations.as_ref().unwrap().plural.as_ref().unwrap();
        assert!(
            plural_map
                .get(&PluralCategory::One)
                .unwrap()
                .string_unit
                .as_ref()
                .unwrap()
                .value
                .contains("%1$@")
        );
    }
}
