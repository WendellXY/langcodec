use langcodec::Codec;
use langcodec::converter::{convert, convert_resources_to_format};
use langcodec::formats::FormatType;
use langcodec::types::{Entry, EntryStatus, Metadata, Resource, Translation};
use proptest::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

fn key_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-z][a-z0-9_]{0,15}").expect("valid key regex")
}

fn value_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[A-Za-z0-9 _\\-\\.,!\\?]{1,30}").expect("valid value regex")
}

fn single_lang_dataset_strategy() -> impl Strategy<Value = BTreeMap<String, String>> {
    prop::collection::btree_map(key_strategy(), value_strategy(), 1..8)
}

fn two_lang_dataset_strategy() -> impl Strategy<Value = BTreeMap<String, (String, String)>> {
    prop::collection::btree_map(key_strategy(), (value_strategy(), value_strategy()), 1..8)
}

fn build_resource(language: &str, values: &BTreeMap<String, String>) -> Resource {
    let mut custom = HashMap::new();
    custom.insert("source_language".to_string(), "en".to_string());
    custom.insert("version".to_string(), "1.0".to_string());

    let entries = values
        .iter()
        .map(|(key, value)| Entry {
            id: key.clone(),
            value: Translation::Singular(value.clone()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        })
        .collect();

    Resource {
        metadata: Metadata {
            language: language.to_string(),
            domain: "Localizable".to_string(),
            custom,
        },
        entries,
    }
}

fn build_two_lang_resources(values: &BTreeMap<String, (String, String)>) -> Vec<Resource> {
    let en_map = values
        .iter()
        .map(|(key, (en, _))| (key.clone(), en.clone()))
        .collect::<BTreeMap<_, _>>();
    let fr_map = values
        .iter()
        .map(|(key, (_, fr))| (key.clone(), fr.clone()))
        .collect::<BTreeMap<_, _>>();

    vec![build_resource("en", &en_map), build_resource("fr", &fr_map)]
}

fn expected_single_lang_map(
    values: &BTreeMap<String, String>,
) -> BTreeMap<(String, String), String> {
    values
        .iter()
        .map(|(key, value)| (("en".to_string(), key.clone()), value.clone()))
        .collect()
}

fn expected_two_lang_map(
    values: &BTreeMap<String, (String, String)>,
) -> BTreeMap<(String, String), String> {
    let mut out = BTreeMap::new();
    for (key, (en, fr)) in values {
        out.insert(("en".to_string(), key.clone()), en.clone());
        out.insert(("fr".to_string(), key.clone()), fr.clone());
    }
    out
}

fn read_resources(path: &Path, lang_hint: Option<&str>) -> Result<Vec<Resource>, String> {
    let mut codec = Codec::new();
    codec
        .read_file_by_extension(path, lang_hint.map(|lang| lang.to_string()))
        .map_err(|e| e.to_string())?;
    Ok(codec.resources)
}

fn canonical_singular_map(resources: &[Resource]) -> BTreeMap<(String, String), String> {
    let mut out = BTreeMap::new();

    for resource in resources {
        for entry in &resource.entries {
            if let Translation::Singular(value) = &entry.value {
                out.insert(
                    (resource.metadata.language.clone(), entry.id.clone()),
                    value.clone(),
                );
            }
        }
    }

    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn strings_android_strings_roundtrip_preserves_singular_entries(values in single_lang_dataset_strategy()) {
        let tmp = tempfile::tempdir().map_err(|e| TestCaseError::fail(e.to_string()))?;
        let input = tmp.path().join("seed.strings");
        let middle = tmp.path().join("middle.xml");
        let output = tmp.path().join("roundtrip.strings");

        let seed = vec![build_resource("en", &values)];
        convert_resources_to_format(
            seed,
            input.to_str().expect("path to str"),
            FormatType::Strings(Some("en".to_string())),
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        convert(
            &input,
            FormatType::Strings(Some("en".to_string())),
            &middle,
            FormatType::AndroidStrings(Some("en".to_string())),
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        convert(
            &middle,
            FormatType::AndroidStrings(Some("en".to_string())),
            &output,
            FormatType::Strings(Some("en".to_string())),
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        let actual = read_resources(&output, Some("en")).map_err(TestCaseError::fail)?;
        prop_assert_eq!(
            canonical_singular_map(&actual),
            expected_single_lang_map(&values)
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn android_strings_android_roundtrip_preserves_singular_entries(values in single_lang_dataset_strategy()) {
        let tmp = tempfile::tempdir().map_err(|e| TestCaseError::fail(e.to_string()))?;
        let input = tmp.path().join("seed.xml");
        let middle = tmp.path().join("middle.strings");
        let output = tmp.path().join("roundtrip.xml");

        let seed = vec![build_resource("en", &values)];
        convert_resources_to_format(
            seed,
            input.to_str().expect("path to str"),
            FormatType::AndroidStrings(Some("en".to_string())),
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        convert(
            &input,
            FormatType::AndroidStrings(Some("en".to_string())),
            &middle,
            FormatType::Strings(Some("en".to_string())),
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        convert(
            &middle,
            FormatType::Strings(Some("en".to_string())),
            &output,
            FormatType::AndroidStrings(Some("en".to_string())),
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        let actual = read_resources(&output, Some("en")).map_err(TestCaseError::fail)?;
        prop_assert_eq!(
            canonical_singular_map(&actual),
            expected_single_lang_map(&values)
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn strings_xcstrings_strings_roundtrip_preserves_singular_entries(values in single_lang_dataset_strategy()) {
        let tmp = tempfile::tempdir().map_err(|e| TestCaseError::fail(e.to_string()))?;
        let input = tmp.path().join("seed.strings");
        let middle = tmp.path().join("middle.xcstrings");
        let output = tmp.path().join("roundtrip.strings");

        let seed = vec![build_resource("en", &values)];
        convert_resources_to_format(
            seed,
            input.to_str().expect("path to str"),
            FormatType::Strings(Some("en".to_string())),
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        convert(
            &input,
            FormatType::Strings(Some("en".to_string())),
            &middle,
            FormatType::Xcstrings,
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        convert(
            &middle,
            FormatType::Xcstrings,
            &output,
            FormatType::Strings(Some("en".to_string())),
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        let actual = read_resources(&output, Some("en")).map_err(TestCaseError::fail)?;
        prop_assert_eq!(
            canonical_singular_map(&actual),
            expected_single_lang_map(&values)
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn csv_xcstrings_csv_roundtrip_preserves_multilang_entries(values in two_lang_dataset_strategy()) {
        let tmp = tempfile::tempdir().map_err(|e| TestCaseError::fail(e.to_string()))?;
        let input = tmp.path().join("seed.csv");
        let middle = tmp.path().join("middle.xcstrings");
        let output = tmp.path().join("roundtrip.csv");

        let seed = build_two_lang_resources(&values);
        convert_resources_to_format(
            seed,
            input.to_str().expect("path to str"),
            FormatType::CSV,
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        convert(&input, FormatType::CSV, &middle, FormatType::Xcstrings)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        convert(&middle, FormatType::Xcstrings, &output, FormatType::CSV)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        let actual = read_resources(&output, None).map_err(TestCaseError::fail)?;
        prop_assert_eq!(canonical_singular_map(&actual), expected_two_lang_map(&values));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn csv_tsv_csv_roundtrip_preserves_multilang_entries(values in two_lang_dataset_strategy()) {
        let tmp = tempfile::tempdir().map_err(|e| TestCaseError::fail(e.to_string()))?;
        let input = tmp.path().join("seed.csv");
        let middle = tmp.path().join("middle.tsv");
        let output = tmp.path().join("roundtrip.csv");

        let seed = build_two_lang_resources(&values);
        convert_resources_to_format(
            seed,
            input.to_str().expect("path to str"),
            FormatType::CSV,
        )
        .map_err(|e| TestCaseError::fail(e.to_string()))?;

        convert(&input, FormatType::CSV, &middle, FormatType::TSV)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        convert(&middle, FormatType::TSV, &output, FormatType::CSV)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        let actual = read_resources(&output, None).map_err(TestCaseError::fail)?;
        prop_assert_eq!(canonical_singular_map(&actual), expected_two_lang_map(&values));
    }
}
