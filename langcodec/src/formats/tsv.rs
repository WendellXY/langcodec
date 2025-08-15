//! Support for TSV (Tab-Separated Values) localization format.
//!
//! Supports multi-language format where the first column is the key and subsequent columns are translations.
//! Only singular key-value pairs are supported; plurals will be dropped during conversion.
//! Provides parsing, serialization, and conversion to/from the internal `Resource` model.
use std::{collections::HashMap, io::BufRead};

use crate::{
    error::Error,
    traits::Parser,
    types::{Entry, EntryStatus, Metadata, Resource, Translation},
};

/// Represents a multi-language TSV record where the first column is the key
/// and subsequent columns are translations for different languages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiLanguageTSVRecord {
    pub key: String,
    pub translations: HashMap<String, String>,
}

impl MultiLanguageTSVRecord {
    /// Creates a new multi-language TSV record.
    pub fn new(key: String) -> Self {
        Self {
            key,
            translations: HashMap::new(),
        }
    }

    /// Adds a translation for a specific language.
    pub fn add_translation(&mut self, language: String, value: String) {
        self.translations.insert(language, value);
    }

    /// Gets a translation for a specific language.
    pub fn get_translation(&self, language: &str) -> Option<&String> {
        self.translations.get(language)
    }
}

/// Represents the TSV format containing all records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    pub records: Vec<MultiLanguageTSVRecord>,
}

impl Format {
    /// Creates a new TSV format with empty records.
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Creates a new TSV format with the given records.
    pub fn with_records(records: Vec<MultiLanguageTSVRecord>) -> Self {
        Self { records }
    }

    /// Adds a record to the format.
    pub fn add_record(&mut self, record: MultiLanguageTSVRecord) {
        self.records.push(record);
    }

    /// Gets all records.
    pub fn get_records(&self) -> &[MultiLanguageTSVRecord] {
        &self.records
    }

    /// Gets all records as mutable.
    pub fn get_records_mut(&mut self) -> &mut [MultiLanguageTSVRecord] {
        &mut self.records
    }
}

impl Default for Format {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser for Format {
    /// Parse from any reader, automatically detecting single vs multi-language format.
    fn from_reader<R: BufRead>(reader: R) -> Result<Self, Error> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b'\t')
            .from_reader(reader);

        let mut records = Vec::new();
        let mut lines = rdr.records();

        // Read the first line to determine the format
        if let Some(first_line) = lines.next() {
            let first_line = first_line.map_err(Error::CsvParse)?;

            if first_line.len() == 2 {
                // Single language format: key, value
                // First line is data, not header
                records.push(MultiLanguageTSVRecord {
                    key: first_line[0].to_string(),
                    translations: {
                        let mut map = HashMap::new();
                        map.insert("default".to_string(), first_line[1].to_string());
                        map
                    },
                });

                // Process remaining lines
                for line in lines {
                    let line = line.map_err(Error::CsvParse)?;
                    if line.len() == 2 {
                        let mut record = MultiLanguageTSVRecord::new(line[0].to_string());
                        record.add_translation("default".to_string(), line[1].to_string());
                        records.push(record);
                    }
                }
            } else if first_line.len() >= 3 {
                // Multi-language format: key, lang1, lang2, ...
                let languages: Vec<String> =
                    first_line.iter().skip(1).map(|s| s.to_string()).collect();

                // First line is header, process remaining lines as data
                for line in lines {
                    let line = line.map_err(Error::CsvParse)?;
                    if line.len() >= 2 {
                        let mut record = MultiLanguageTSVRecord::new(line[0].to_string());
                        for (i, lang) in languages.iter().enumerate() {
                            if i + 1 < line.len() {
                                record.add_translation(lang.clone(), line[i + 1].to_string());
                            }
                        }
                        records.push(record);
                    }
                }
            } else {
                return Err(Error::DataMismatch(
                    "Invalid TSV format: insufficient columns".to_string(),
                ));
            }
        }

        Ok(Format { records })
    }

    /// Write to any writer (file, memory, etc.).
    fn to_writer<W: std::io::Write>(&self, writer: W) -> Result<(), Error> {
        if self.records.is_empty() {
            return Ok(());
        }

        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b'\t')
            .from_writer(writer);

        // Get all unique languages from all records
        let mut all_languages = std::collections::HashSet::new();
        for record in &self.records {
            for lang in record.translations.keys() {
                all_languages.insert(lang.clone());
            }
        }

        // Sort languages for consistent output
        let mut sorted_languages: Vec<String> = all_languages.into_iter().collect();
        sorted_languages.sort();

        // Write header row
        let mut header = vec!["key".to_string()];
        header.extend(sorted_languages.clone());
        wtr.write_record(&header).map_err(Error::CsvParse)?;

        // Write data rows
        for record in &self.records {
            let mut row = vec![record.key.clone()];
            let empty_string = String::new();
            for lang in &sorted_languages {
                let value = record.translations.get(lang).unwrap_or(&empty_string);
                row.push(value.clone());
            }
            wtr.write_record(&row).map_err(Error::CsvParse)?;
        }

        wtr.flush().map_err(Error::Io)?;
        Ok(())
    }
}

impl TryFrom<Vec<Resource>> for Format {
    type Error = Error;

    fn try_from(resources: Vec<Resource>) -> Result<Self, Self::Error> {
        if resources.is_empty() {
            return Ok(Format::new());
        }

        // Get all unique keys across all resources
        let mut all_keys = std::collections::HashSet::new();
        for resource in &resources {
            for entry in &resource.entries {
                all_keys.insert(entry.id.clone());
            }
        }

        // Create a multi-language record for each key
        let mut records = Vec::new();
        for key in all_keys {
            let mut record = MultiLanguageTSVRecord::new(key);

            for resource in &resources {
                if let Some(entry) = resource.entries.iter().find(|e| e.id == record.key) {
                    let value = match &entry.value {
                        Translation::Singular(v) => v.clone(),
                        Translation::Plural(_) => String::new(), // Plurals not supported
                    };
                    record.add_translation(resource.metadata.language.clone(), value);
                }
            }

            records.push(record);
        }

        Ok(Format { records })
    }
}

impl TryFrom<Format> for Vec<Resource> {
    type Error = Error;

    fn try_from(format: Format) -> Result<Self, Self::Error> {
        if format.records.is_empty() {
            return Ok(Vec::new());
        }

        // Get all unique languages
        let mut all_languages = std::collections::HashSet::new();
        for record in &format.records {
            for lang in record.translations.keys() {
                all_languages.insert(lang.clone());
            }
        }

        // Create a resource for each language
        let mut resources = Vec::new();
        let mut custom_metadata = HashMap::new();
        
        // Add required metadata for XCStrings compatibility
        // Use the first language as source language, or "en" as default
        let source_language = all_languages.iter().next().unwrap_or(&"en".to_string()).clone();
        custom_metadata.insert("source_language".to_string(), source_language);
        custom_metadata.insert("version".to_string(), "1.0".to_string());
        
        for language in all_languages {
            let mut resource = Resource {
                metadata: Metadata {
                    language: language.clone(),
                    domain: String::from(""),
                    custom: custom_metadata.clone(),
                },
                entries: Vec::new(),
            };

            for record in &format.records {
                if let Some(translation) = record.translations.get(&language) {
                    resource.entries.push(Entry {
                        id: record.key.clone(),
                        value: Translation::Singular(translation.clone()),
                        comment: None,
                        status: EntryStatus::Translated,
                        custom: HashMap::new(),
                    });
                }
            }

            if !resource.entries.is_empty() {
                resources.push(resource);
            }
        }

        Ok(resources)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Parser;
    use crate::types::{Resource, Translation};
    use std::io::Cursor;

    #[test]
    fn test_parse_simple_tsv() {
        let tsv_content = "hello\tHello\nbye\tGoodbye\n";
        let format = Format::from_reader(Cursor::new(tsv_content)).unwrap();
        assert_eq!(format.records.len(), 2);
        assert_eq!(format.records[0].key, "hello");
        assert_eq!(
            format.records[0].get_translation("default"),
            Some(&"Hello".to_string())
        );
        assert_eq!(format.records[1].key, "bye");
        assert_eq!(
            format.records[1].get_translation("default"),
            Some(&"Goodbye".to_string())
        );
    }

    #[test]
    fn test_round_trip_tsv_resource_tsv() {
        let tsv_content = "hello\tHello\nbye\tGoodbye\n";
        let format = Format::from_reader(Cursor::new(tsv_content)).unwrap();
        let resources = Vec::<Resource>::try_from(format.clone()).unwrap();
        let serialized: Format = TryFrom::try_from(resources).unwrap();

        // Sort records by key for comparison since order may not be guaranteed
        let mut original_records = format.records.clone();
        let mut serialized_records = serialized.records.clone();
        original_records.sort_by(|a, b| a.key.cmp(&b.key));
        serialized_records.sort_by(|a, b| a.key.cmp(&b.key));

        assert_eq!(original_records, serialized_records);
    }

    #[test]
    fn test_tsv_row_with_empty_value() {
        let tsv_content = "empty\t\nhello\tHello\n";
        let format = Format::from_reader(Cursor::new(tsv_content)).unwrap();
        assert_eq!(format.records.len(), 2);
        assert_eq!(format.records[0].key, "empty");
        assert_eq!(
            format.records[0].get_translation("default"),
            Some(&"".to_string())
        );
        let resources = Vec::<Resource>::try_from(format.clone()).unwrap();
        assert_eq!(resources.len(), 1);
        // The entry with empty value should be present and its value should be empty
        let resource = &resources[0];
        assert_eq!(resource.entries.len(), 2);
        let entry = &resource.entries[0];
        assert_eq!(entry.id, "empty");
        assert_eq!(
            match &entry.value {
                Translation::Singular(s) => s,
                _ => panic!("Expected singular translation"),
            },
            ""
        );
    }

    #[test]
    fn test_tsv_with_tabs_in_values() {
        let tsv_content = "key1\tValue with tabs\nkey2\tAnother value\n";
        let format = Format::from_reader(Cursor::new(tsv_content)).unwrap();

        // The CSV parser with tab delimiter creates one record per row
        // Each row is split into key-value pairs based on the first tab
        // So we get 2 records total
        assert_eq!(format.records.len(), 2);

        // First row: key1 -> Value with tabs
        assert_eq!(format.records[0].key, "key1");
        assert_eq!(
            format.records[0].get_translation("default"),
            Some(&"Value with tabs".to_string())
        );

        // Second row: key2 -> Another value
        assert_eq!(format.records[1].key, "key2");
        assert_eq!(
            format.records[1].get_translation("default"),
            Some(&"Another value".to_string())
        );
    }

    #[test]
    fn test_parse_multi_language_tsv() {
        let tsv_content = "key\ten\tcn\nhello\tHello\t你好\nbye\tGoodbye\t再见\n";
        let format = Format::from_reader(Cursor::new(tsv_content)).unwrap();
        assert_eq!(format.records.len(), 2);

        // Check first record (first data row after header)
        assert_eq!(format.records[0].key, "hello");
        assert_eq!(
            format.records[0].get_translation("en"),
            Some(&"Hello".to_string())
        );
        assert_eq!(
            format.records[0].get_translation("cn"),
            Some(&"你好".to_string())
        );

        // Check second record
        assert_eq!(format.records[1].key, "bye");
        assert_eq!(
            format.records[1].get_translation("en"),
            Some(&"Goodbye".to_string())
        );
        assert_eq!(
            format.records[1].get_translation("cn"),
            Some(&"再见".to_string())
        );
    }

    #[test]
    fn test_parse_single_language_tsv_as_multi() {
        let tsv_content = "hello\tHello\nbye\tGoodbye\n";
        let format = Format::from_reader(Cursor::new(tsv_content)).unwrap();
        assert_eq!(format.records.len(), 2);

        // Check first record
        assert_eq!(format.records[0].key, "hello");
        assert_eq!(
            format.records[0].get_translation("default"),
            Some(&"Hello".to_string())
        );

        // Check second record
        assert_eq!(format.records[1].key, "bye");
        assert_eq!(
            format.records[1].get_translation("default"),
            Some(&"Goodbye".to_string())
        );
    }

    #[test]
    fn test_multi_language_tsv_to_resources() {
        let tsv_content = "key\ten\tcn\nhello\tHello\t你好\nbye\tGoodbye\t再见\n";
        let format = Format::from_reader(Cursor::new(tsv_content)).unwrap();
        let resources = Vec::<Resource>::try_from(format).unwrap();

        assert_eq!(resources.len(), 2);

        // Check English resource
        let en_resource = resources
            .iter()
            .find(|r| r.metadata.language == "en")
            .unwrap();
        assert_eq!(en_resource.entries.len(), 2);
        assert_eq!(en_resource.entries[0].id, "hello");
        assert_eq!(en_resource.entries[1].id, "bye");

        // Check Chinese resource
        let cn_resource = resources
            .iter()
            .find(|r| r.metadata.language == "cn")
            .unwrap();
        assert_eq!(cn_resource.entries.len(), 2);
        assert_eq!(cn_resource.entries[0].id, "hello");
        assert_eq!(cn_resource.entries[1].id, "bye");
    }

    #[test]
    fn test_write_multi_language_tsv() {
        let mut record1 = MultiLanguageTSVRecord::new("hello".to_string());
        record1.add_translation("en".to_string(), "Hello".to_string());
        record1.add_translation("cn".to_string(), "你好".to_string());

        let mut record2 = MultiLanguageTSVRecord::new("bye".to_string());
        record2.add_translation("en".to_string(), "Goodbye".to_string());
        record2.add_translation("cn".to_string(), "再见".to_string());

        let records = vec![record1, record2];

        let mut output = Vec::new();
        Format::with_records(records)
            .to_writer(&mut output)
            .unwrap();
        let output_str = String::from_utf8(output).unwrap();

        // The output should have a header row and data rows
        let lines: Vec<&str> = output_str.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 data rows

        // Check header contains key, en, cn (sorted)
        assert!(lines[0].contains("key"));
        assert!(lines[0].contains("cn"));
        assert!(lines[0].contains("en"));
    }

    #[test]
    fn test_multi_language_tsv_record_methods() {
        let mut record = MultiLanguageTSVRecord::new("test_key".to_string());

        // Test initial state
        assert_eq!(record.key, "test_key");
        assert_eq!(record.translations.len(), 0);
        assert_eq!(record.get_translation("en"), None);

        // Test adding translations
        record.add_translation("en".to_string(), "Hello".to_string());
        record.add_translation("cn".to_string(), "你好".to_string());
        record.add_translation("es".to_string(), "Hola".to_string());

        // Test getting translations
        assert_eq!(record.get_translation("en"), Some(&"Hello".to_string()));
        assert_eq!(record.get_translation("cn"), Some(&"你好".to_string()));
        assert_eq!(record.get_translation("es"), Some(&"Hola".to_string()));
        assert_eq!(record.get_translation("fr"), None);

        // Test updating existing translation
        record.add_translation("en".to_string(), "Updated Hello".to_string());
        assert_eq!(
            record.get_translation("en"),
            Some(&"Updated Hello".to_string())
        );

        // Test translations count
        assert_eq!(record.translations.len(), 3);
    }

    #[test]
    fn test_multi_language_tsv_record_clone() {
        let mut record1 = MultiLanguageTSVRecord::new("key1".to_string());
        record1.add_translation("en".to_string(), "Hello".to_string());
        record1.add_translation("cn".to_string(), "你好".to_string());

        let record2 = record1.clone();

        assert_eq!(record1.key, record2.key);
        assert_eq!(record1.translations, record2.translations);
        assert_eq!(record1.get_translation("en"), record2.get_translation("en"));
        assert_eq!(record1.get_translation("cn"), record2.get_translation("cn"));
    }

    #[test]
    fn test_multi_language_tsv_record_debug() {
        let mut record = MultiLanguageTSVRecord::new("test_key".to_string());
        record.add_translation("en".to_string(), "Hello".to_string());
        record.add_translation("cn".to_string(), "你好".to_string());

        let debug_str = format!("{:?}", record);
        assert!(debug_str.contains("MultiLanguageTSVRecord"));
        assert!(debug_str.contains("test_key"));
        assert!(debug_str.contains("Hello"));
        assert!(debug_str.contains("你好"));
    }

    #[test]
    fn test_multi_language_tsv_record_partial_eq() {
        let mut record1 = MultiLanguageTSVRecord::new("key1".to_string());
        record1.add_translation("en".to_string(), "Hello".to_string());
        record1.add_translation("cn".to_string(), "你好".to_string());

        let mut record2 = MultiLanguageTSVRecord::new("key1".to_string());
        record2.add_translation("en".to_string(), "Hello".to_string());
        record2.add_translation("cn".to_string(), "你好".to_string());

        let mut record3 = MultiLanguageTSVRecord::new("key2".to_string());
        record3.add_translation("en".to_string(), "Hello".to_string());

        assert_eq!(record1, record2);
        assert_ne!(record1, record3);
        assert_ne!(record2, record3);
    }

    #[test]
    fn test_multi_language_tsv_record_empty_translations() {
        let record = MultiLanguageTSVRecord::new("empty_key".to_string());

        assert_eq!(record.key, "empty_key");
        assert_eq!(record.translations.len(), 0);
        assert_eq!(record.get_translation("en"), None);
        assert_eq!(record.get_translation("cn"), None);
    }

    #[test]
    fn test_multi_language_tsv_record_unicode_keys() {
        let mut record = MultiLanguageTSVRecord::new("测试键".to_string());
        record.add_translation("en".to_string(), "Test Key".to_string());
        record.add_translation("cn".to_string(), "测试键".to_string());

        assert_eq!(record.key, "测试键");
        assert_eq!(record.get_translation("en"), Some(&"Test Key".to_string()));
        assert_eq!(record.get_translation("cn"), Some(&"测试键".to_string()));
    }

    #[test]
    fn test_multi_language_tsv_record_special_characters() {
        let mut record = MultiLanguageTSVRecord::new("key_with_special_chars".to_string());
        record.add_translation("en".to_string(), "Hello, World!".to_string());
        record.add_translation("cn".to_string(), "你好，世界！".to_string());
        record.add_translation("es".to_string(), "¡Hola, mundo!".to_string());

        assert_eq!(
            record.get_translation("en"),
            Some(&"Hello, World!".to_string())
        );
        assert_eq!(
            record.get_translation("cn"),
            Some(&"你好，世界！".to_string())
        );
        assert_eq!(
            record.get_translation("es"),
            Some(&"¡Hola, mundo!".to_string())
        );
    }

    #[test]
    fn test_multi_language_tsv_record_overwrite_translation() {
        let mut record = MultiLanguageTSVRecord::new("overwrite_test".to_string());

        // Add initial translation
        record.add_translation("en".to_string(), "Original".to_string());
        assert_eq!(record.get_translation("en"), Some(&"Original".to_string()));

        // Overwrite with new translation
        record.add_translation("en".to_string(), "Updated".to_string());
        assert_eq!(record.get_translation("en"), Some(&"Updated".to_string()));
        assert_eq!(record.translations.len(), 1); // Should still be only one entry
    }

    #[test]
    fn test_multi_language_tsv_record_multiple_languages() {
        let mut record = MultiLanguageTSVRecord::new("multilingual".to_string());

        let languages = vec![
            ("en", "English"),
            ("cn", "中文"),
            ("es", "Español"),
            ("fr", "Français"),
            ("de", "Deutsch"),
            ("ja", "日本語"),
            ("ko", "한국어"),
            ("ru", "Русский"),
        ];

        for (code, translation) in &languages {
            record.add_translation(code.to_string(), translation.to_string());
        }

        assert_eq!(record.translations.len(), 8);

        for (code, translation) in languages {
            assert_eq!(record.get_translation(code), Some(&translation.to_string()));
        }
    }

    #[test]
    fn test_multi_language_tsv_record_tab_in_values() {
        let mut record = MultiLanguageTSVRecord::new("tab_test".to_string());
        record.add_translation("en".to_string(), "Value\twith\ttabs".to_string());
        record.add_translation("cn".to_string(), "带\t制表符\t的值".to_string());

        assert_eq!(
            record.get_translation("en"),
            Some(&"Value\twith\ttabs".to_string())
        );
        assert_eq!(
            record.get_translation("cn"),
            Some(&"带\t制表符\t的值".to_string())
        );
    }

    #[test]
    fn test_multi_language_tsv_record_newlines_in_values() {
        let mut record = MultiLanguageTSVRecord::new("newline_test".to_string());
        record.add_translation("en".to_string(), "Line 1\nLine 2".to_string());
        record.add_translation("cn".to_string(), "第一行\n第二行".to_string());

        assert_eq!(
            record.get_translation("en"),
            Some(&"Line 1\nLine 2".to_string())
        );
        assert_eq!(
            record.get_translation("cn"),
            Some(&"第一行\n第二行".to_string())
        );
    }

    #[test]
    fn test_multi_language_tsv_record_case_sensitivity() {
        let mut record = MultiLanguageTSVRecord::new("case_test".to_string());
        record.add_translation("EN".to_string(), "English".to_string());
        record.add_translation("en".to_string(), "english".to_string());
        record.add_translation("En".to_string(), "English".to_string());

        assert_eq!(record.get_translation("EN"), Some(&"English".to_string()));
        assert_eq!(record.get_translation("en"), Some(&"english".to_string()));
        assert_eq!(record.get_translation("En"), Some(&"English".to_string()));
        assert_eq!(record.translations.len(), 3);
    }

    #[test]
    fn test_tsv_language_key_preservation() {
        // Create a TSV with specific language keys
        let tsv_content = "key\ten\tfr\tde\nhello\tHello\tBonjour\tHallo\nbye\tGoodbye\tAu revoir\tAuf Wiedersehen\n";
        let format = Format::from_reader(Cursor::new(tsv_content)).unwrap();

        // Check that the language keys are preserved
        assert_eq!(format.records.len(), 2);

        // Check first record
        let first_record = &format.records[0];
        assert_eq!(first_record.key, "hello");
        assert_eq!(
            first_record.get_translation("en"),
            Some(&"Hello".to_string())
        );
        assert_eq!(
            first_record.get_translation("fr"),
            Some(&"Bonjour".to_string())
        );
        assert_eq!(
            first_record.get_translation("de"),
            Some(&"Hallo".to_string())
        );

        // Check second record
        let second_record = &format.records[1];
        assert_eq!(second_record.key, "bye");
        assert_eq!(
            second_record.get_translation("en"),
            Some(&"Goodbye".to_string())
        );
        assert_eq!(
            second_record.get_translation("fr"),
            Some(&"Au revoir".to_string())
        );
        assert_eq!(
            second_record.get_translation("de"),
            Some(&"Auf Wiedersehen".to_string())
        );
    }

    #[test]
    fn test_tsv_to_resources_language_preservation() {
        // Create a TSV with specific language keys
        let tsv_content = "key\ten\tfr\tde\nhello\tHello\tBonjour\tHallo\nbye\tGoodbye\tAu revoir\tAuf Wiedersehen\n";
        let format = Format::from_reader(Cursor::new(tsv_content)).unwrap();

        // Convert to resources
        let resources = Vec::<Resource>::try_from(format).unwrap();

        // Check that we have resources for each language
        assert_eq!(resources.len(), 3);

        // Check English resource
        let en_resource = resources
            .iter()
            .find(|r| r.metadata.language == "en")
            .unwrap();
        assert_eq!(en_resource.entries.len(), 2);
        assert_eq!(en_resource.entries[0].id, "hello");
        assert_eq!(
            en_resource.entries[0].value,
            Translation::Singular("Hello".to_string())
        );
        assert_eq!(en_resource.entries[1].id, "bye");
        assert_eq!(
            en_resource.entries[1].value,
            Translation::Singular("Goodbye".to_string())
        );

        // Check French resource
        let fr_resource = resources
            .iter()
            .find(|r| r.metadata.language == "fr")
            .unwrap();
        assert_eq!(fr_resource.entries.len(), 2);
        assert_eq!(en_resource.entries[0].id, "hello");
        assert_eq!(
            fr_resource.entries[0].value,
            Translation::Singular("Bonjour".to_string())
        );
        assert_eq!(fr_resource.entries[1].id, "bye");
        assert_eq!(
            fr_resource.entries[1].value,
            Translation::Singular("Au revoir".to_string())
        );

        // Check German resource
        let de_resource = resources
            .iter()
            .find(|r| r.metadata.language == "de")
            .unwrap();
        assert_eq!(de_resource.entries.len(), 2);
        assert_eq!(de_resource.entries[0].id, "hello");
        assert_eq!(
            de_resource.entries[0].value,
            Translation::Singular("Hallo".to_string())
        );
        assert_eq!(de_resource.entries[1].id, "bye");
        assert_eq!(
            de_resource.entries[1].value,
            Translation::Singular("Auf Wiedersehen".to_string())
        );
    }

    #[test]
    fn test_tsv_round_trip_language_preservation() {
        // Create a TSV with specific language keys
        let tsv_content = "key\ten\tfr\tde\nhello\tHello\tBonjour\tHallo\nbye\tGoodbye\tAu revoir\tAuf Wiedersehen\n";
        let original_format = Format::from_reader(Cursor::new(tsv_content)).unwrap();

        // Convert to resources and back to TSV
        let resources = Vec::<Resource>::try_from(original_format.clone()).unwrap();
        let round_trip_format = Format::try_from(resources).unwrap();

        // Check that language keys are preserved in round trip
        assert_eq!(
            original_format.records.len(),
            round_trip_format.records.len()
        );

        // Sort records by key for comparison
        let mut original_records = original_format.records.clone();
        let mut round_trip_records = round_trip_format.records.clone();
        original_records.sort_by(|a, b| a.key.cmp(&b.key));
        round_trip_records.sort_by(|a, b| a.key.cmp(&b.key));

        for (original, round_trip) in original_records.iter().zip(round_trip_records.iter()) {
            assert_eq!(original.key, round_trip.key);
            assert_eq!(original.translations, round_trip.translations);
        }
    }
}
