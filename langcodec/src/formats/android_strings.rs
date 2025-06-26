//! Support for Android `strings.xml` localization format.
//!
//! Only singular `<string>` elements are supported. Plurals (`<plurals>`) are not yet implemented.
//! Provides parsing, serialization, and conversion to/from the internal `Resource` model.

use quick_xml::{
    Reader, Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{BufRead, Write},
};

use crate::{
    error::Error,
    traits::Parser,
    types::{Entry, EntryStatus, Metadata, Resource, Translation},
};

#[derive(Debug, Serialize)]
pub struct Format {
    pub language: String,
    pub strings: Vec<StringResource>,
}

impl Parser for Format {
    /// Parse from any reader.
    fn from_reader<R: BufRead>(reader: R) -> Result<Self, Error> {
        let mut xml_reader = Reader::from_reader(reader);
        xml_reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut resources = Vec::new();

        loop {
            match xml_reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"string" => {
                    let sr = parse_string_resource(e, &mut xml_reader)?;
                    resources.push(sr);
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => return Err(Error::XmlParse(e)),
            }
            buf.clear();
        }
        Ok(Format {
            language: String::new(), // strings.xml does not contain language metadata
            strings: resources,
        })
    }

    /// Write to any writer (file, memory, etc.).
    fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        let mut xml_writer = Writer::new(&mut writer);

        xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
        xml_writer.write_event(Event::Text(BytesText::new("\n")))?;

        let resources_start = BytesStart::new("resources");
        xml_writer.write_event(Event::Start(resources_start))?;
        xml_writer.write_event(Event::Text(BytesText::new("\n")))?;

        for sr in &self.strings {
            let mut elem = BytesStart::new("string");
            elem.push_attribute(("name", sr.name.as_str()));
            if let Some(trans) = sr.translatable {
                elem.push_attribute(("translatable", if trans { "true" } else { "false" }));
            }

            xml_writer.write_event(Event::Start(elem))?;
            xml_writer.write_event(Event::Text(BytesText::new(&sr.value)))?;
            xml_writer.write_event(Event::End(BytesEnd::new("string")))?;
            xml_writer.write_event(Event::Text(BytesText::new("\n")))?;
        }

        xml_writer.write_event(Event::End(BytesEnd::new("resources")))?;
        xml_writer.write_event(Event::Text(BytesText::new("\n")))?;
        Ok(())
    }
}

impl From<Resource> for Format {
    fn from(value: Resource) -> Self {
        Self {
            language: value.metadata.language.clone(),
            strings: value
                .entries
                .iter()
                .map(StringResource::from_entry)
                .collect(),
        }
    }
}

impl From<Format> for Resource {
    fn from(value: Format) -> Self {
        Resource {
            metadata: Metadata {
                language: value.language.clone(),
                domain: String::new(), // strings.xml does not have a domain
                custom: HashMap::new(),
            },
            entries: value.strings.iter().map(StringResource::to_entry).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StringResource {
    pub name: String,
    pub value: String,
    pub translatable: Option<bool>,
}

impl StringResource {
    fn to_entry(&self) -> Entry {
        Entry {
            id: self.name.clone(),
            value: Translation::Singular(self.value.clone()),
            comment: None,
            status: match self.translatable {
                Some(true) => EntryStatus::Translated,
                Some(false) => EntryStatus::DoNotTranslate,
                None if self.value.is_empty() => EntryStatus::New,
                None => EntryStatus::Translated,
            },
            custom: HashMap::new(),
        }
    }

    fn from_entry(entry: &Entry) -> Self {
        StringResource {
            name: entry.id.clone(),
            value: match &entry.value {
                Translation::Singular(v) => v.clone(),
                Translation::Plural(_) => String::new(), // Plurals not supported in strings.xml
            },
            translatable: match entry.status {
                EntryStatus::Translated => Some(true),
                EntryStatus::DoNotTranslate => Some(false),
                EntryStatus::New => None,
                _ => None, // Other statuses not applicable
            },
        }
    }
}

fn parse_string_resource<R: BufRead>(
    e: &BytesStart,
    xml_reader: &mut Reader<R>,
) -> Result<StringResource, Error> {
    let mut name = None;
    let mut translatable = None;

    for attr in e.attributes().with_checks(false) {
        let attr = attr.map_err(|e| Error::DataMismatch(e.to_string()))?;
        match attr.key.as_ref() {
            b"name" => name = Some(attr.unescape_value()?.to_string()),
            b"translatable" => {
                let v = attr.unescape_value()?.to_string();
                translatable = Some(v == "true");
            }
            _ => {}
        }
    }
    let name =
        name.ok_or_else(|| Error::InvalidResource("string tag missing 'name'".to_string()))?;

    let mut buf = Vec::new();
    // Read until text or end
    let value = loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                let v = e.unescape().map_err(Error::XmlParse)?.to_string();
                break v;
            }
            Ok(Event::End(_)) => break String::new(),
            Ok(Event::Eof) => return Err(Error::InvalidResource("Unexpected EOF".to_string())),
            Ok(_) => (),
            Err(e) => return Err(Error::XmlParse(e)),
        }
        buf.clear();
    };
    Ok(StringResource {
        name,
        value,
        translatable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Parser;
    use crate::types::EntryStatus;

    #[test]
    fn test_parse_basic_strings_xml() {
        let xml = r#"
        <resources>
            <string name="hello">Hello</string>
            <string name="bye" translatable="false">Goodbye</string>
            <string name="empty"></string>
        </resources>
        "#;
        let format = Format::from_str(xml).unwrap();
        assert_eq!(format.strings.len(), 3);
        let hello = &format.strings[0];
        assert_eq!(hello.name, "hello");
        assert_eq!(hello.value, "Hello");
        assert_eq!(hello.translatable, None); // no attribute
        let bye = &format.strings[1];
        assert_eq!(bye.name, "bye");
        assert_eq!(bye.value, "Goodbye");
        assert_eq!(bye.translatable, Some(false));
        let empty = &format.strings[2];
        assert_eq!(empty.name, "empty");
        assert_eq!(empty.value, "");
        assert_eq!(empty.translatable, None);
    }

    #[test]
    fn test_parse_plurals_ignored() {
        let xml = r#"
        <resources>
            <string name="hello">Hello</string>
            <plurals name="apples">
                <item quantity="one">One apple</item>
                <item quantity="other">%d apples</item>
            </plurals>
        </resources>
        "#;
        // Plurals are ignored (not parsed as string resources), so only 1 entry.
        let format = Format::from_str(xml).unwrap();
        assert_eq!(format.strings.len(), 1);
        assert_eq!(format.strings[0].name, "hello");
    }

    #[test]
    fn test_missing_name_attribute() {
        let xml = r#"
        <resources>
            <string>No name attr</string>
        </resources>
        "#;
        let result = Format::from_str(xml);
        assert!(result.is_err());
        let err = format!("{:?}", result.unwrap_err());
        assert!(err.contains("missing 'name'"));
    }

    #[test]
    fn test_round_trip_serialization() {
        let xml = r#"
        <resources>
            <string name="greet">Hi</string>
            <string name="bye" translatable="false">Bye</string>
        </resources>
        "#;
        let format = Format::from_str(xml).unwrap();
        let mut out = Vec::new();
        format.to_writer(&mut out).unwrap();
        let out_str = String::from_utf8(out).unwrap();
        let reparsed = Format::from_str(&out_str).unwrap();
        assert_eq!(format.strings.len(), reparsed.strings.len());
        for (orig, new) in format.strings.iter().zip(reparsed.strings.iter()) {
            assert_eq!(orig.name, new.name);
            assert_eq!(orig.value, new.value);
            assert_eq!(orig.translatable, new.translatable);
        }
    }

    #[test]
    fn test_entry_with_empty_value_status_new() {
        let xml = r#"
        <resources>
            <string name="empty"></string>
        </resources>
        "#;
        let format = Format::from_str(xml).unwrap();
        assert_eq!(format.strings.len(), 1);
        let entry = format.strings[0].to_entry();
        assert_eq!(entry.status, EntryStatus::New);
    }
}
