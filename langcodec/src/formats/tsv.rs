//! Support for TSV (Tab-Separated Values) localization format.
//!
//! Supports both single language key-value pairs and multi-language formats.
//! For multi-language format, the first column is the key and subsequent columns are translations.
//! Only singular key-value pairs are supported; plurals will be dropped during conversion.
//! Provides parsing, serialization, and conversion to/from the internal `Resource` model.
use std::{collections::HashMap, io::BufRead};

use serde::{Deserialize, Serialize};

use crate::{
    error::Error,
    traits::Parser,
    types::{Entry, EntryStatus, Metadata, Resource, Translation},
};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct TSVRecord {
    pub key: String,
    pub value: String,
}

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

impl Parser for Vec<TSVRecord> {
    /// Parse from any reader.
    fn from_reader<R: BufRead>(reader: R) -> Result<Self, Error> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b'\t')
            .from_reader(reader);
        let mut records = Vec::new();
        for result in rdr.deserialize() {
            records.push(result?);
        }
        Ok(records)
    }

    /// Write to any writer (file, memory, etc.).
    fn to_writer<W: std::io::Write>(&self, writer: W) -> Result<(), Error> {
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b'\t')
            .from_writer(writer);
        for record in self {
            wtr.serialize(record)?;
        }
        wtr.flush()?;
        Ok(())
    }
}

impl Parser for Vec<MultiLanguageTSVRecord> {
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

            if first_line.len() >= 3 {
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
            } else if first_line.len() == 2 {
                // Single language format: key, value
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
            } else {
                return Err(Error::DataMismatch(
                    "Invalid TSV format: insufficient columns".to_string(),
                ));
            }
        }

        Ok(records)
    }

    /// Write to any writer (file, memory, etc.).
    fn to_writer<W: std::io::Write>(&self, writer: W) -> Result<(), Error> {
        if self.is_empty() {
            return Ok(());
        }

        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b'\t')
            .from_writer(writer);

        // Get all unique languages from all records
        let mut all_languages = std::collections::HashSet::new();
        for record in self {
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
        for record in self {
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

impl From<Vec<TSVRecord>> for Resource {
    fn from(value: Vec<TSVRecord>) -> Self {
        Resource {
            metadata: Metadata {
                language: String::from(""),
                domain: String::from(""),
                custom: HashMap::new(),
            },
            entries: value
                .into_iter()
                .map(|record| {
                    Entry {
                        id: record.key,
                        value: Translation::Singular(record.value),
                        comment: None,
                        status: EntryStatus::Translated, // Default status
                        custom: HashMap::new(),
                    }
                })
                .collect(),
        }
    }
}

impl TryFrom<Resource> for Vec<TSVRecord> {
    type Error = Error;

    fn try_from(value: Resource) -> Result<Self, Self::Error> {
        Ok(value
            .entries
            .into_iter()
            .map(|entry| TSVRecord {
                key: entry.id.clone(),
                value: match entry.value {
                    Translation::Singular(v) => v,
                    Translation::Plural(_) => String::new(), // Plurals not supported in TSV
                },
            })
            .collect())
    }
}

// Helper function to convert MultiLanguageTSVRecord to Vec<Resource>
pub fn multi_language_tsv_to_resources(records: Vec<MultiLanguageTSVRecord>) -> Vec<Resource> {
    if records.is_empty() {
        return Vec::new();
    }

    // Get all unique languages
    let mut all_languages = std::collections::HashSet::new();
    for record in &records {
        for lang in record.translations.keys() {
            all_languages.insert(lang.clone());
        }
    }

    // Create a resource for each language
    let mut resources = Vec::new();
    for language in all_languages {
        let mut resource = Resource {
            metadata: Metadata {
                language: language.clone(),
                domain: String::from(""),
                custom: HashMap::new(),
            },
            entries: Vec::new(),
        };

        for record in &records {
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

    resources
}

// Helper function to convert Vec<Resource> to MultiLanguageTSVRecord
pub fn resources_to_multi_language_tsv(
    resources: &[Resource],
) -> Result<Vec<MultiLanguageTSVRecord>, Error> {
    if resources.is_empty() {
        return Ok(Vec::new());
    }

    // Get all unique keys across all resources
    let mut all_keys = std::collections::HashSet::new();
    for resource in resources {
        for entry in &resource.entries {
            all_keys.insert(entry.id.clone());
        }
    }

    // Create a multi-language record for each key
    let mut records = Vec::new();
    for key in all_keys {
        let mut record = MultiLanguageTSVRecord::new(key);

        for resource in resources {
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

    Ok(records)
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
        let records = Vec::<TSVRecord>::from_reader(Cursor::new(tsv_content)).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].key, "hello");
        assert_eq!(records[0].value, "Hello");
        assert_eq!(records[1].key, "bye");
        assert_eq!(records[1].value, "Goodbye");
    }

    #[test]
    fn test_round_trip_tsv_resource_tsv() {
        let tsv_content = "hello\tHello\nbye\tGoodbye\n";
        let records = Vec::<TSVRecord>::from_reader(Cursor::new(tsv_content)).unwrap();
        let resource = Resource::from(records.clone());
        let serialized: Vec<TSVRecord> = TryFrom::try_from(resource).unwrap();
        // Should be the same key-value pairs (order may not be guaranteed, but for this test, it is)
        assert_eq!(records, serialized);
    }

    #[test]
    fn test_tsv_row_with_empty_value() {
        let tsv_content = "empty\t\nhello\tHello\n";
        let records = Vec::<TSVRecord>::from_reader(Cursor::new(tsv_content)).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].key, "empty");
        assert_eq!(records[0].value, "");
        let resource = Resource::from(records.clone());
        assert_eq!(resource.entries.len(), 2);
        // The entry with empty value should be present and its value should be empty
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
        let tsv_content = "key1\tValue with\t tabs\nkey2\tAnother\tvalue\n";
        let records = Vec::<TSVRecord>::from_reader(Cursor::new(tsv_content)).unwrap();

        // The CSV parser with tab delimiter creates one record per row
        // Each row is split into key-value pairs based on the first tab
        // So we get 2 records total
        assert_eq!(records.len(), 2);

        // First row: key1 -> Value with
        assert_eq!(records[0].key, "key1");
        assert_eq!(records[0].value, "Value with");

        // Second row: key2 -> Another
        assert_eq!(records[1].key, "key2");
        assert_eq!(records[1].value, "Another");
    }

    #[test]
    fn test_parse_multi_language_tsv() {
        let tsv_content = "key\ten\tcn\nhello\tHello\t你好\nbye\tGoodbye\t再见\n";
        let records = Vec::<MultiLanguageTSVRecord>::from_reader(Cursor::new(tsv_content)).unwrap();
        assert_eq!(records.len(), 2);

        // Check first record (first data row after header)
        assert_eq!(records[0].key, "hello");
        assert_eq!(records[0].get_translation("en"), Some(&"Hello".to_string()));
        assert_eq!(records[0].get_translation("cn"), Some(&"你好".to_string()));

        // Check second record
        assert_eq!(records[1].key, "bye");
        assert_eq!(
            records[1].get_translation("en"),
            Some(&"Goodbye".to_string())
        );
        assert_eq!(records[1].get_translation("cn"), Some(&"再见".to_string()));
    }

    #[test]
    fn test_parse_single_language_tsv_as_multi() {
        let tsv_content = "hello\tHello\nbye\tGoodbye\n";
        let records = Vec::<MultiLanguageTSVRecord>::from_reader(Cursor::new(tsv_content)).unwrap();
        assert_eq!(records.len(), 2);

        // Check first record
        assert_eq!(records[0].key, "hello");
        assert_eq!(
            records[0].get_translation("default"),
            Some(&"Hello".to_string())
        );

        // Check second record
        assert_eq!(records[1].key, "bye");
        assert_eq!(
            records[1].get_translation("default"),
            Some(&"Goodbye".to_string())
        );
    }

    #[test]
    fn test_multi_language_tsv_to_resources() {
        let tsv_content = "key\ten\tcn\nhello\tHello\t你好\nbye\tGoodbye\t再见\n";
        let records = Vec::<MultiLanguageTSVRecord>::from_reader(Cursor::new(tsv_content)).unwrap();
        let resources = multi_language_tsv_to_resources(records);

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
        records.to_writer(&mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();

        // The output should have a header row and data rows
        let lines: Vec<&str> = output_str.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 data rows

        // Check header contains key, en, cn (sorted)
        assert!(lines[0].contains("key"));
        assert!(lines[0].contains("cn"));
        assert!(lines[0].contains("en"));
    }
}
