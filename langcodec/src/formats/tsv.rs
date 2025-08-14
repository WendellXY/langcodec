//! Support for TSV (Tab-Separated Values) localization format.
//!
//! Only singular key-value pairs are supported; plurals will be dropped during conversion.
//! Provides parsing, serialization, and conversion to/from the internal `Resource` model.
//! Note: TSV format only supports singular translations; plurals will be dropped.
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
}
