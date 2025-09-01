use std::collections::BTreeSet;

use unic_langid::LanguageIdentifier;

use crate::{
    error::Error,
    types::{Plural, PluralCategory, Resource, Translation},
};

/// Returns the required CLDR plural categories for a given language identifier.
///
/// This is a curated subset of CLDR rules covering common locales. For unknown
/// or unsupported locales, falls back to {Other} to avoid false positives.
pub fn required_categories_for(lang: &LanguageIdentifier) -> BTreeSet<PluralCategory> {
    let mut set: BTreeSet<PluralCategory> = BTreeSet::new();

    // Base language subtag only for rule selection
    let lang_str = lang.language.as_str();

    match lang_str {
        // One/Other languages (most European languages)
        "en" | "de" | "nl" | "sv" | "da" | "nb" | "nn" | "no" | "is" | "fi" | "et"
        | "fa" | "hi" | "bn" | "gu" | "ta" | "te" | "kn" | "ml" | "mr" | "it"
        | "es" | "pt" | "pt_br" | "pt_pt" | "mk" | "el" | "eu" | "gl" | "af" | "sw"
        | "ur" | "fil" | "tl" | "tr" | "id" | "ms" => {
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Other);
        }

        // Only Other (East Asian languages and some SE Asian)
        "ja" | "zh" | "ko" | "th" | "vi" | "km" | "lo" | "my" | "yue" | "zh_hant"
        | "zh_hans" => {
            set.insert(PluralCategory::Other);
        }

        // French-like (CLDR: one/other)
        "fr" | "hy" | "kab" => {
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Other);
        }

        // Slavic (Russian group): one, few, many, other
        "ru" | "uk" | "be" | "sr" | "hr" | "bs" | "sh" => {
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Few);
            set.insert(PluralCategory::Many);
            set.insert(PluralCategory::Other);
        }

        // Polish: one, few, many, other
        "pl" => {
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Few);
            set.insert(PluralCategory::Many);
            set.insert(PluralCategory::Other);
        }

        // Czech/Slovak: one, few, other
        "cs" | "sk" => {
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Few);
            set.insert(PluralCategory::Other);
        }

        // Slovenian: one, two, few, other
        "sl" => {
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Two);
            set.insert(PluralCategory::Few);
            set.insert(PluralCategory::Other);
        }

        // Lithuanian: one, few, other
        "lt" => {
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Few);
            set.insert(PluralCategory::Other);
        }

        // Latvian: zero, one, other
        "lv" => {
            set.insert(PluralCategory::Zero);
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Other);
        }

        // Irish Gaelic: one, two, few, many, other
        "ga" => {
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Two);
            set.insert(PluralCategory::Few);
            set.insert(PluralCategory::Many);
            set.insert(PluralCategory::Other);
        }

        // Romanian: one, few, other
        "ro" => {
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Few);
            set.insert(PluralCategory::Other);
        }

        // Arabic: zero, one, two, few, many, other
        "ar" => {
            set.insert(PluralCategory::Zero);
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Two);
            set.insert(PluralCategory::Few);
            set.insert(PluralCategory::Many);
            set.insert(PluralCategory::Other);
        }

        // Hebrew (cardinals) commonly use one, two, many, other in CLDR
        "iw" /* legacy */ | "he" => {
            set.insert(PluralCategory::One);
            set.insert(PluralCategory::Two);
            set.insert(PluralCategory::Many);
            set.insert(PluralCategory::Other);
        }

        _ => {
            // Conservative default to avoid noisy validation for unknown locales
            set.insert(PluralCategory::Other);
        }
    }

    set
}

/// Helper for string language codes (accepts underscores, normalizes to hyphen).
pub fn required_categories_for_str(lang: &str) -> BTreeSet<PluralCategory> {
    let normalized = lang.replace('_', "-");
    let parsed: LanguageIdentifier = normalized.parse().unwrap_or_else(|_| "und".parse().unwrap());
    required_categories_for(&parsed)
}

/// Compute which required categories are missing for a given plural entry and language.
pub fn missing_categories_for_plural(
    lang: &LanguageIdentifier,
    plural: &Plural,
) -> BTreeSet<PluralCategory> {
    let required = required_categories_for(lang);
    let have: BTreeSet<PluralCategory> = plural.forms.keys().cloned().collect();
    &required - &have
}

/// Validate a single resource for missing plural categories.
pub fn validate_resource_plurals(resource: &Resource) -> Result<(), Error> {
    let lang_id = match resource.parse_language_identifier() {
        Some(id) => id,
        None => {
            return Err(Error::validation_error(format!(
                "Invalid or missing language for resource: {}",
                resource.metadata.language
            )));
        }
    };

    let mut problems: Vec<String> = Vec::new();

    for entry in &resource.entries {
        if let Translation::Plural(plural) = &entry.value {
            let missing = missing_categories_for_plural(&lang_id, plural);
            if !missing.is_empty() {
                let have: Vec<String> = plural
                    .forms
                    .keys()
                    .map(|k| format!("{:?}", k))
                    .collect();
                let miss: Vec<String> = missing.into_iter().map(|k| format!("{:?}", k)).collect();
                problems.push(format!(
                    "lang='{}' key='{}': missing plural categories: [{}] (have: [{}])",
                    resource.metadata.language,
                    entry.id,
                    miss.join(", "),
                    have.join(", ")
                ));
            }
        }
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(Error::validation_error(format!(
            "Plural validation failed:\n{}",
            problems.join("\n")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Entry, EntryStatus, Metadata};

    #[test]
    fn test_required_categories_basic() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        let ru: LanguageIdentifier = "ru".parse().unwrap();
        let ja: LanguageIdentifier = "ja".parse().unwrap();

        let en_set = required_categories_for(&en);
        assert!(en_set.contains(&PluralCategory::One));
        assert!(en_set.contains(&PluralCategory::Other));
        assert_eq!(en_set.len(), 2);

        let ru_set = required_categories_for(&ru);
        assert!(ru_set.contains(&PluralCategory::One));
        assert!(ru_set.contains(&PluralCategory::Few));
        assert!(ru_set.contains(&PluralCategory::Many));
        assert!(ru_set.contains(&PluralCategory::Other));
        assert_eq!(ru_set.len(), 4);

        let ja_set = required_categories_for(&ja);
        assert!(ja_set.contains(&PluralCategory::Other));
        assert_eq!(ja_set.len(), 1);
    }

    #[test]
    fn test_validate_resource_plurals_missing() {
        // English requires one/other; missing 'one' should fail
        let resource = Resource {
            metadata: Metadata {
                language: "en".into(),
                domain: String::new(),
                custom: Default::default(),
            },
            entries: vec![Entry {
                id: "apples".into(),
                value: Translation::Plural(Plural::new(
                    "apples",
                    vec![(PluralCategory::Other, "%d apples".to_string())].into_iter(),
                )
                .unwrap()),
                comment: None,
                status: EntryStatus::Translated,
                custom: Default::default(),
            }],
        };

        let err = validate_resource_plurals(&resource).unwrap_err();
        assert!(format!("{}", err).contains("missing plural categories"));
    }
}
