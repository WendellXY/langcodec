//! Core, format-agnostic types for langcodec.
//! Parsers decode into these; encoders serialize these.

use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    str::FromStr,
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use unic_langid::LanguageIdentifier;

use crate::{error::Error, traits::Parser};

impl Parser for Vec<Resource> {
    /// Parse from any reader.
    fn from_reader<R: std::io::BufRead>(reader: R) -> Result<Self, Error> {
        serde_json::from_reader(reader).map_err(Error::Parse)
    }

    /// Write to any writer (file, memory, etc.).
    fn to_writer<W: std::io::Write>(&self, mut writer: W) -> Result<(), Error> {
        serde_json::to_writer(&mut writer, self).map_err(Error::Parse)
    }
}

/// A complete localization resource (corresponds to a `.strings`, `.xml`, `.xcstrings`, etc. file).
/// Contains metadata and all entries for a single language and domain.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Resource {
    /// Optional header-level metadata (language code, domain/project, etc.).
    pub metadata: Metadata,

    /// Ordered list of all entries in this resource.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<Entry>,
}

impl Resource {
    pub(crate) fn add_entry(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub fn parse_language_identifier(&self) -> Option<LanguageIdentifier> {
        self.metadata.language.parse().ok()
    }
}

/// Free-form metadata for the resource as a whole.
///
/// `language` and `domain` are standard; any extra fields can be placed in `custom`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Metadata {
    /// The language code (e.g. "en", "fr", "es", etc.).
    pub language: String,

    /// The domain or project name (e.g. "MyApp").
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub domain: String,

    /// Any other metadata fields not covered by the above.
    pub custom: HashMap<String, String>,
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map_all = self.custom.clone();
        map_all.insert("language".to_string(), self.language.clone());
        map_all.insert("domain".to_string(), self.domain.clone());
        write!(
            f,
            "Metadata {{ {} }}",
            map_all
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// A single message/translation entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Entry {
    /// Unique message identifier (key).  
    /// For PO/XLIFF this is `msgid` or `<trans-unit>@id`; for .strings it’s the key.
    pub id: String,

    /// Translation context corresponding to this message.
    pub value: Translation,

    /// Optional comment for translators.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub comment: Option<String>,

    /// Entry translation status.
    pub status: EntryStatus,

    /// Any additional, format-specific data attached to this entry.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Entry {{ id: {}, value: {}, status: {:?} }}",
            self.id, self.value, self.status
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Translation {
    /// A single translation without plural forms.
    Singular(String),

    /// A translation with plural forms.
    Plural(Plural),
}

impl Translation {
    pub fn plain_translation(translation: Translation) -> Translation {
        match translation {
            Translation::Singular(value) => {
                Translation::Singular(make_plain_translation_string(value))
            }
            Translation::Plural(plural) => {
                // Return the first plural form as a singular translation
                let id = plural.id;
                let forms = plural.forms.into_iter().next().map_or_else(
                    || BTreeMap::new(),
                    |(category, value)| {
                        let mut map = BTreeMap::new();
                        map.insert(category, make_plain_translation_string(value));
                        map
                    },
                );
                Translation::Plural(Plural { id, forms })
            }
        }
    }

    pub fn plain_translation_string(&self) -> String {
        match self {
            Translation::Singular(value) => make_plain_translation_string(value.clone()),
            Translation::Plural(plural) => {
                // Return the first plural form as a singular translation
                plural.forms.values().next().map_or_else(
                    || String::new(),
                    |value| make_plain_translation_string(value.clone()),
                )
            }
        }
    }
}

impl Display for Translation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Translation::Singular(value) => write!(f, "{}", value),
            Translation::Plural(plural) => write!(f, "{}", plural.id), // Displaying only the ID for brevity
        }
    }
}

/// All plural forms for a single message.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Plural {
    /// The canonical plural ID (`msgid_plural` in PO).
    pub id: String,

    /// Map from category → translation.  
    /// Categories depend on the target locale’s rules.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(default)]
    pub forms: BTreeMap<PluralCategory, String>,
}

impl Plural {
    pub(crate) fn new(
        id: &str,
        forms: impl Iterator<Item = (PluralCategory, String)>,
    ) -> Option<Self> {
        let forms: BTreeMap<PluralCategory, String> = forms.collect();

        if forms.is_empty() {
            None // No plural forms provided
        } else {
            Some(Self {
                id: id.to_string(),
                forms,
            })
        }
    }
}

/// Standard CLDR plural forms.
#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[derive(Hash)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl FromStr for PluralCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ZERO" => Ok(PluralCategory::Zero),
            "ONE" => Ok(PluralCategory::One),
            "TWO" => Ok(PluralCategory::Two),
            "FEW" => Ok(PluralCategory::Few),
            "MANY" => Ok(PluralCategory::Many),
            "OTHER" => Ok(PluralCategory::Other),
            _ => Err(format!("Unknown plural category: {}", s)),
        }
    }
}

/// Status of a translation entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryStatus {
    /// The entry is not translated and should not be.
    DoNotTranslate,

    /// The entry is new and has not been translated yet.
    New,

    /// The entry is outdated.
    Stale,

    /// The entry has been modified and needs review.
    NeedsReview,

    /// The entry is translated and reviewed.
    Translated,
}

impl FromStr for EntryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "DO_NOT_TRANSLATE" => Ok(EntryStatus::DoNotTranslate),
            "NEW" => Ok(EntryStatus::New),
            "STALE" => Ok(EntryStatus::Stale),
            "NEEDS_REVIEW" => Ok(EntryStatus::NeedsReview),
            "TRANSLATED" => Ok(EntryStatus::Translated),
            _ => Err(format!("Unknown entry status: {}", s)),
        }
    }
}

// Remove HTML tags from translation string.
fn make_plain_translation_string(translation: String) -> String {
    let mut translation = translation;
    translation = translation.trim().to_string();

    // Remove all HTML tags (non-greedy)
    let re_html = Regex::new(r"<[^>]+>").unwrap();
    translation = re_html.replace_all(&translation, "").to_string();

    // Remove all closing tags like </font>
    let re_html_close = Regex::new(r"</[^>]+>").unwrap();
    translation = re_html_close.replace_all(&translation, "").to_string();

    // Replace any newline characters with explicit "\n" for better formatting,
    translation = translation
        .lines()
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join(r"\n"); // Use r"\n" for a literal \n

    translation
}
