// Note: CSV format only supports singular translations; plurals will be dropped.
use std::{collections::HashMap, io::BufRead};

use serde::{Deserialize, Serialize};

use crate::{
    error::Error,
    traits::Parser,
    types::{Entry, EntryStatus, Metadata, Resource, Translation},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct CSVRecord {
    pub key: String,
    pub value: String,
}

impl Parser for Vec<CSVRecord> {
    /// Parse from any reader.
    fn from_reader<R: BufRead>(reader: R) -> Result<Self, Error> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(reader);
        let mut records = Vec::new();
        for result in rdr.deserialize() {
            records.push(result?);
        }
        Ok(records)
    }

    /// Write to any writer (file, memory, etc.).
    fn to_writer<W: std::io::Write>(&self, writer: W) -> Result<(), Error> {
        let mut wtr = csv::WriterBuilder::new().from_writer(writer);
        for record in self {
            wtr.serialize(record)?;
        }
        wtr.flush()?;
        Ok(())
    }
}

impl From<Vec<CSVRecord>> for Resource {
    fn from(value: Vec<CSVRecord>) -> Self {
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

impl TryFrom<Resource> for Vec<CSVRecord> {
    type Error = Error;

    fn try_from(value: Resource) -> Result<Self, Self::Error> {
        Ok(value
            .entries
            .into_iter()
            .map(|entry| CSVRecord {
                key: entry.id.clone(),
                value: match entry.value {
                    Translation::Singular(v) => v,
                    Translation::Plural(_) => String::new(), // Plurals not supported in CSV
                },
            })
            .collect())
    }
}
