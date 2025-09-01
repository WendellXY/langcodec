use std::collections::{BTreeMap, BTreeSet};

use unic_langid::LanguageIdentifier;

use crate::{
    error::Error,
    types::{Plural, PluralCategory, Resource, Translation},
};

use lazy_static::lazy_static;

lazy_static! {
    /// Static mapping from base language subtag → required plural categories (CLDR‑style, cardinals).
    static ref CATEGORY_TABLE: BTreeMap<&'static str, BTreeSet<PluralCategory>> = {
        use PluralCategory::*;
        let mut m: BTreeMap<&'static str, BTreeSet<PluralCategory>> = BTreeMap::new();

        // Helper to build a set from a slice
        fn s(items: &[PluralCategory]) -> BTreeSet<PluralCategory> {
            items.iter().cloned().collect()
        }

        // One/Other (most Indo‑European languages without complex plural rules)
        for code in [
            "en","de","nl","sv","da","nb","nn","no","is","fi","et","fa","hi","bn","gu",
            "ta","te","kn","ml","mr","it","es","pt","mk","el","eu","gl","af","sw","ur",
            "fil","tl","tr","id","ms","fr","hy","kab"
        ] {
            m.insert(code, s(&[One, Other]));
        }

        // Only Other (East/Southeast Asian common cases)
        for code in ["ja","zh","ko","th","vi","km","lo","my","yue"] {
            m.insert(code, s(&[Other]));
        }

        // Slavic (Russian group): one, few, many, other
        for code in ["ru","uk","be","sr","hr","bs","sh"] {
            m.insert(code, s(&[One, Few, Many, Other]));
        }

        // Polish
        m.insert("pl", s(&[One, Few, Many, Other]));

        // Czech/Slovak
        for code in ["cs","sk"] {
            m.insert(code, s(&[One, Few, Other]));
        }

        // Slovenian
        m.insert("sl", s(&[One, Two, Few, Other]));

        // Lithuanian
        m.insert("lt", s(&[One, Few, Other]));

        // Latvian
        m.insert("lv", s(&[Zero, One, Other]));

        // Irish Gaelic
        m.insert("ga", s(&[One, Two, Few, Many, Other]));

        // Romanian
        m.insert("ro", s(&[One, Few, Other]));

        // Arabic
        m.insert("ar", s(&[Zero, One, Two, Few, Many, Other]));

        // Hebrew (legacy code iw also maps here)
        for code in ["he","iw"] {
            m.insert(code, s(&[One, Two, Many, Other]));
        }

        m
    };
}

/// Returns the required CLDR plural categories for a given language identifier.
///
/// This is a curated subset of CLDR rules covering common locales. For unknown
/// or unsupported locales, falls back to {Other} to avoid false positives.
pub fn required_categories_for(lang: &LanguageIdentifier) -> BTreeSet<PluralCategory> {
    // Base language subtag only for rule selection
    let lang_str = lang.language.as_str();
    CATEGORY_TABLE
        .get(lang_str)
        .cloned()
        .unwrap_or_else(|| {
            // Conservative default to avoid noisy validation for unknown locales
            let mut s = BTreeSet::new();
            s.insert(PluralCategory::Other);
            s
        })
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
