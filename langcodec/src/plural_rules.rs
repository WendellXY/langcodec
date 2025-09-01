use std::collections::{BTreeMap, BTreeSet};

use unic_langid::LanguageIdentifier;

use crate::{
    error::Error,
    types::{EntryStatus, Plural, PluralCategory, Resource, Translation},
};

use lazy_static::lazy_static;
use serde::Serialize;

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

/// Non-fatal report describing missing plural categories for a key in a locale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluralValidationReport {
    pub language: String,
    pub key: String,
    pub missing: BTreeSet<PluralCategory>,
    pub have: BTreeSet<PluralCategory>,
}

/// Returns the required CLDR plural categories for a given language identifier.
///
/// This is a curated subset of CLDR rules covering common locales. For unknown
/// or unsupported locales, falls back to {Other} to avoid false positives.
pub fn required_categories_for(lang: &LanguageIdentifier) -> BTreeSet<PluralCategory> {
    // Base language subtag only for rule selection
    let lang_str = lang.language.as_str();
    CATEGORY_TABLE.get(lang_str).cloned().unwrap_or_else(|| {
        // Conservative default to avoid noisy validation for unknown locales
        let mut s = BTreeSet::new();
        s.insert(PluralCategory::Other);
        s
    })
}

/// Helper for string language codes (accepts underscores, normalizes to hyphen).
pub fn required_categories_for_str(lang: &str) -> BTreeSet<PluralCategory> {
    let normalized = lang.replace('_', "-");
    let parsed: LanguageIdentifier = normalized
        .parse()
        .unwrap_or_else(|_| "und".parse().unwrap());
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

/// Collect non-fatal plural issues for a single resource.
pub fn collect_resource_plural_issues(resource: &Resource) -> Vec<PluralValidationReport> {
    let Some(lang_id) = resource.parse_language_identifier() else {
        return vec![PluralValidationReport {
            language: resource.metadata.language.clone(),
            key: String::from("<resource>"),
            missing: [PluralCategory::Other].into_iter().collect(),
            have: BTreeSet::new(),
        }];
    };

    let mut reports = Vec::new();
    for entry in &resource.entries {
        if let Translation::Plural(plural) = &entry.value {
            let have: BTreeSet<PluralCategory> = plural.forms.keys().cloned().collect();
            let missing = missing_categories_for_plural(&lang_id, plural);
            if !missing.is_empty() {
                reports.push(PluralValidationReport {
                    language: resource.metadata.language.clone(),
                    key: entry.id.clone(),
                    missing,
                    have,
                });
            }
        }
    }
    reports
}

/// Validate a single resource for missing plural categories.
pub fn validate_resource_plurals(resource: &Resource) -> Result<(), Error> {
    let reports = collect_resource_plural_issues(resource);
    if reports.is_empty() {
        return Ok(());
    }
    let mut lines = Vec::new();
    for r in reports {
        let miss: Vec<String> = r.missing.iter().map(|k| format!("{:?}", k)).collect();
        let have: Vec<String> = r.have.iter().map(|k| format!("{:?}", k)).collect();
        lines.push(format!(
            "lang='{}' key='{}': missing plural categories: [{}] (have: [{}])",
            r.language,
            r.key,
            miss.join(", "),
            have.join(", ")
        ));
    }
    Err(Error::validation_error(format!(
        "Plural validation failed:\n{}",
        lines.join("\n")
    )))
}

/// Autofix: for each plural entry, fill missing categories using the 'other' value if available.
/// Marks the entry status as NeedsReview when any categories are added. Skips DoNotTranslate.
///
/// Returns the number of categories added across the resource.
pub fn autofix_fill_missing_from_other_resource(resource: &mut Resource) -> usize {
    let Some(lang_id) = resource.parse_language_identifier() else {
        return 0;
    };
    let mut added = 0usize;
    for entry in &mut resource.entries {
        // Skip entries that shouldn't be translated
        if matches!(entry.status, EntryStatus::DoNotTranslate) {
            continue;
        }
        if let Translation::Plural(plural) = &mut entry.value {
            let missing = missing_categories_for_plural(&lang_id, plural);
            if missing.is_empty() {
                continue;
            }
            // Need an 'other' form to duplicate
            if let Some(other_val) = plural.forms.get(&PluralCategory::Other).cloned() {
                let mut added_here = 0usize;
                for cat in missing {
                    // Insert only if still missing (avoid race with duplicates)
                    if let std::collections::btree_map::Entry::Vacant(e) = plural.forms.entry(cat) {
                        e.insert(other_val.clone());
                        added += 1;
                        added_here += 1;
                    }
                }
                // Mark as needs review if anything was added
                if added_here > 0 && !matches!(entry.status, EntryStatus::NeedsReview) {
                    entry.status = EntryStatus::NeedsReview;
                }
            }
        }
    }
    added
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
                value: Translation::Plural(
                    Plural::new(
                        "apples",
                        vec![(PluralCategory::Other, "%d apples".to_string())].into_iter(),
                    )
                    .unwrap(),
                ),
                comment: None,
                status: EntryStatus::Translated,
                custom: Default::default(),
            }],
        };

        let err = validate_resource_plurals(&resource).unwrap_err();
        assert!(format!("{}", err).contains("missing plural categories"));
    }

    #[test]
    fn test_collect_resource_plural_issues() {
        // English requires one/other; missing 'one' should yield a report
        let resource = Resource {
            metadata: Metadata {
                language: "en".into(),
                domain: String::new(),
                custom: Default::default(),
            },
            entries: vec![Entry {
                id: "apples".into(),
                value: Translation::Plural(
                    Plural::new(
                        "apples",
                        vec![(PluralCategory::Other, "%d apples".to_string())].into_iter(),
                    )
                    .unwrap(),
                ),
                comment: None,
                status: EntryStatus::Translated,
                custom: Default::default(),
            }],
        };

        let reports = collect_resource_plural_issues(&resource);
        assert_eq!(reports.len(), 1);
        let r = &reports[0];
        assert_eq!(r.language, "en");
        assert_eq!(r.key, "apples");
        assert!(r.missing.contains(&PluralCategory::One));
        assert!(r.have.contains(&PluralCategory::Other));
    }

    #[test]
    fn test_autofix_fill_missing_from_other_resource() {
        // English requires one/other; provide only other and autofix should add one
        let mut resource = Resource {
            metadata: Metadata {
                language: "en".into(),
                domain: String::new(),
                custom: Default::default(),
            },
            entries: vec![Entry {
                id: "apples".into(),
                value: Translation::Plural(
                    Plural::new(
                        "apples",
                        vec![(PluralCategory::Other, "%d apples".to_string())].into_iter(),
                    )
                    .unwrap(),
                ),
                comment: None,
                status: EntryStatus::Translated,
                custom: Default::default(),
            }],
        };

        let added = autofix_fill_missing_from_other_resource(&mut resource);
        assert!(added >= 1);
        let entry = &resource.entries[0];
        // Should now contain One and Other
        if let Translation::Plural(p) = &entry.value {
            assert!(p.forms.contains_key(&PluralCategory::One));
            assert_eq!(
                p.forms.get(&PluralCategory::One).unwrap(),
                p.forms.get(&PluralCategory::Other).unwrap()
            );
        } else {
            panic!("expected plural");
        }
        assert!(matches!(entry.status, EntryStatus::NeedsReview));
    }

    #[test]
    fn test_autofix_does_not_mark_unchanged_entries() {
        // English: first entry missing 'one', second entry already complete
        let mut resource = Resource {
            metadata: Metadata {
                language: "en".into(),
                domain: String::new(),
                custom: Default::default(),
            },
            entries: vec![
                Entry {
                    id: "apples".into(),
                    value: Translation::Plural(
                        Plural::new(
                            "apples",
                            vec![(PluralCategory::Other, "%d apples".to_string())].into_iter(),
                        )
                        .unwrap(),
                    ),
                    comment: None,
                    status: EntryStatus::Translated,
                    custom: Default::default(),
                },
                Entry {
                    id: "bananas".into(),
                    value: Translation::Plural(
                        Plural::new(
                            "bananas",
                            vec![
                                (PluralCategory::One, "One banana".to_string()),
                                (PluralCategory::Other, "%d bananas".to_string()),
                            ]
                            .into_iter(),
                        )
                        .unwrap(),
                    ),
                    comment: None,
                    status: EntryStatus::Translated,
                    custom: Default::default(),
                },
            ],
        };

        let added = autofix_fill_missing_from_other_resource(&mut resource);
        assert!(added >= 1);
        // First entry should be NeedsReview
        assert!(matches!(resource.entries[0].status, EntryStatus::NeedsReview));
        // Second entry should remain Translated
        assert!(matches!(resource.entries[1].status, EntryStatus::Translated));
    }
}
