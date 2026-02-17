use langcodec::traits::Parser;
use langcodec::types::{Entry, EntryStatus, Metadata, Resource, Translation};
use langcodec::{Codec, convert_auto, formats::XcstringsFormat};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MatrixFormat {
    Strings,
    AndroidXml,
    Xcstrings,
    Csv,
    Tsv,
}

impl MatrixFormat {
    fn id(self) -> &'static str {
        match self {
            MatrixFormat::Strings => "strings",
            MatrixFormat::AndroidXml => "android_xml",
            MatrixFormat::Xcstrings => "xcstrings",
            MatrixFormat::Csv => "csv",
            MatrixFormat::Tsv => "tsv",
        }
    }

    fn output_file_name(self, source: MatrixFormat) -> String {
        match self {
            MatrixFormat::Strings => format!("{}_to_strings.strings", source.id()),
            MatrixFormat::AndroidXml => format!("{}_to_strings.xml", source.id()),
            MatrixFormat::Xcstrings => format!("{}_to_localizable.xcstrings", source.id()),
            MatrixFormat::Csv => format!("{}_to_translations.csv", source.id()),
            MatrixFormat::Tsv => format!("{}_to_translations.tsv", source.id()),
        }
    }
}

fn build_seed_resource() -> Resource {
    let mut custom = HashMap::new();
    custom.insert("source_language".to_string(), "en".to_string());
    custom.insert("version".to_string(), "1.0".to_string());

    Resource {
        metadata: Metadata {
            language: "en".to_string(),
            domain: "Localizable".to_string(),
            custom,
        },
        entries: vec![
            Entry {
                id: "hello".to_string(),
                value: Translation::Singular("Hello".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            },
            Entry {
                id: "bye".to_string(),
                value: Translation::Singular("Goodbye".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            },
        ],
    }
}

fn write_seed_files(root: &Path) -> HashMap<MatrixFormat, PathBuf> {
    let strings_dir = root.join("seed").join("en.lproj");
    let android_dir = root.join("seed").join("values-en");
    let generic_dir = root.join("seed");

    std::fs::create_dir_all(&strings_dir).expect("create strings seed dir");
    std::fs::create_dir_all(&android_dir).expect("create android seed dir");
    std::fs::create_dir_all(&generic_dir).expect("create generic seed dir");

    let strings_path = strings_dir.join("Localizable.strings");
    let android_path = android_dir.join("strings.xml");
    let xcstrings_path = generic_dir.join("Localizable.xcstrings");
    let csv_path = generic_dir.join("translations.csv");
    let tsv_path = generic_dir.join("translations.tsv");

    std::fs::write(
        &strings_path,
        "\"hello\" = \"Hello\";\n\"bye\" = \"Goodbye\";\n",
    )
    .expect("write strings seed");
    std::fs::write(
        &android_path,
        "<resources>\n  <string name=\"hello\">Hello</string>\n  <string name=\"bye\">Goodbye</string>\n</resources>\n",
    )
    .expect("write android seed");
    std::fs::write(&csv_path, "key,en\nhello,Hello\nbye,Goodbye\n").expect("write csv seed");
    std::fs::write(&tsv_path, "key\ten\nhello\tHello\nbye\tGoodbye\n").expect("write tsv seed");

    let xcstrings = XcstringsFormat::try_from(vec![build_seed_resource()]).expect("xcstrings seed");
    xcstrings
        .write_to(&xcstrings_path)
        .expect("write xcstrings seed");

    HashMap::from([
        (MatrixFormat::Strings, strings_path),
        (MatrixFormat::AndroidXml, android_path),
        (MatrixFormat::Xcstrings, xcstrings_path),
        (MatrixFormat::Csv, csv_path),
        (MatrixFormat::Tsv, tsv_path),
    ])
}

fn assert_en_entries(path: &Path, case_name: &str) {
    let mut codec = Codec::new();
    codec
        .read_file_by_extension(path, Some("en".to_string()))
        .unwrap_or_else(|e| {
            panic!(
                "{case_name}: failed to read output {}: {}",
                path.display(),
                e
            )
        });

    let en_resource = codec
        .get_by_language("en")
        .unwrap_or_else(|| panic!("{case_name}: expected an 'en' resource"));
    assert!(
        en_resource.entries.len() >= 2,
        "{case_name}: expected at least 2 entries"
    );

    let hello = codec
        .find_entry("hello", "en")
        .unwrap_or_else(|| panic!("{case_name}: missing key 'hello'"));
    match &hello.value {
        Translation::Singular(value) => assert_eq!(value, "Hello", "{case_name}: bad hello value"),
        other => panic!("{case_name}: expected singular hello, got {:?}", other),
    }

    let bye = codec
        .find_entry("bye", "en")
        .unwrap_or_else(|| panic!("{case_name}: missing key 'bye'"));
    match &bye.value {
        Translation::Singular(value) => assert_eq!(value, "Goodbye", "{case_name}: bad bye value"),
        other => panic!("{case_name}: expected singular bye, got {:?}", other),
    }
}

#[test]
fn conversion_matrix_common_paths_preserve_values() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let seed_paths = write_seed_files(temp.path());

    let formats = [
        MatrixFormat::Strings,
        MatrixFormat::AndroidXml,
        MatrixFormat::Xcstrings,
        MatrixFormat::Csv,
        MatrixFormat::Tsv,
    ];

    let output_root = temp.path().join("matrix_output");
    std::fs::create_dir_all(&output_root).expect("create matrix output dir");

    for source in formats {
        for target in formats {
            if source == target {
                continue;
            }

            let source_path = seed_paths
                .get(&source)
                .unwrap_or_else(|| panic!("missing seed path for {:?}", source));
            let target_path = output_root.join(target.output_file_name(source));

            let case_name = format!("{} -> {}", source.id(), target.id());
            convert_auto(source_path, &target_path)
                .unwrap_or_else(|e| panic!("{case_name}: conversion failed: {}", e));
            assert_en_entries(&target_path, &case_name);
        }
    }
}
