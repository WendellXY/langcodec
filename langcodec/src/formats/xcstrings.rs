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

        for resource in resources {
            if let Some(meta_source_language) = resource.metadata.custom.get("source_language") {
                // source_language should be modified if and only if it is empty, otherwise,
                // it should throw an error if it differs.
                if source_language.is_empty() {
                    source_language = meta_source_language.clone();
                } else if source_language != *meta_source_language {
                    return Err(Error::DataMismatch(format!(
                        "Source language mismatch: expected {}, found {}",
                        source_language, meta_source_language
                    )));
                }
            } else {
                return Err(Error::InvalidResource(
                    "No source language found in metadata".to_string(),
                ));
            }

            if let Some(meta_version) = resource.metadata.custom.get("version") {
                // version should be modified if and only if it is empty, otherwise,
                // it should throw an error if it differs.
                if version.is_empty() {
                    version = meta_version.clone();
                } else if version != *meta_version {
                    return Err(Error::DataMismatch(format!(
                        "Version mismatch: expected {}, found {}",
                        version, meta_version
                    )));
                }
            } else {
                return Err(Error::InvalidResource(
                    "No version found in metadata".to_string(),
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
                        })
                        .localizations
                        .extend(item.localizations);
                }
            }
        }

        Ok(Format {
            source_language: source_language,
            version: version,
            strings: strings,
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
    pub localizations: HashMap<String, Localization>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_state: Option<ExtractionState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should_translate: Option<bool>,
}

impl Item {
    fn new(entry: Entry, language: String) -> Option<Self> {
        let mut localizations = HashMap::new();

        let should_translate = Some(entry.status != EntryStatus::DoNotTranslate);

        match entry.value {
            Translation::Singular(value) => {
                localizations.insert(
                    language,
                    Localization {
                        string_unit: Some(StringUnit {
                            state: entry.status,
                            value: value,
                        }),
                        variations: None,
                    },
                );
            }
            Translation::Plural(plural) => {
                let mut plural_map = HashMap::new();
                for (category, value) in plural.forms {
                    plural_map.insert(
                        category,
                        PluralVariation {
                            string_unit: Some(StringUnit {
                                state: entry.status.clone(),
                                value: value,
                            }),
                        },
                    );
                }
                localizations.insert(
                    language,
                    Localization {
                        string_unit: None,
                        variations: Some(Variations {
                            plural: Some(plural_map),
                        }),
                    },
                );
            }
        }

        let extraction_state = entry
            .custom
            .get("extraction_state")?
            .parse::<ExtractionState>()
            .ok();

        Some(Item {
            localizations,
            comment: entry.comment,
            extraction_state,
            should_translate,
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

impl ToString for ExtractionState {
    fn to_string(&self) -> String {
        match self {
            ExtractionState::Manual => "manual".to_string(),
            ExtractionState::Stale => "stale".to_string(),
            ExtractionState::ExtractedWithValue => "extracted_with_value".to_string(),
            ExtractionState::Migrated => "migrated".to_string(),
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Variations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plural: Option<HashMap<PluralCategory, PluralVariation>>,
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
