//! Support for Android `strings.xml` localization format.
//!
//! Supports singular `<string>` and plural `<plurals>` elements.
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
    str::FromStr,
};

use crate::{
    error::Error,
    traits::Parser,
    types::{Entry, EntryStatus, Metadata, Plural, PluralCategory, Resource, Translation},
};

#[derive(Debug, Serialize)]
pub struct Format {
    pub language: String,
    pub strings: Vec<StringResource>,
    pub plurals: Vec<PluralsResource>,
}

impl Parser for Format {
    /// Parse from any reader.
    fn from_reader<R: BufRead>(reader: R) -> Result<Self, Error> {
        let mut xml_reader = Reader::from_reader(reader);
        // Preserve whitespace inside text nodes so multi-line strings and
        // indentation are kept exactly as authored in XML.
        xml_reader.config_mut().trim_text(false);

        let mut buf = Vec::new();
        let mut string_resources = Vec::new();
        let mut plural_resources: Vec<PluralsResource> = Vec::new();

        loop {
            match xml_reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"string" => {
                    let sr = parse_string_resource(e, &mut xml_reader)?;
                    string_resources.push(sr);
                }
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"plurals" => {
                    let pr = parse_plurals_resource(e, &mut xml_reader)?;
                    plural_resources.push(pr);
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => return Err(Error::XmlParse(e)),
            }
            buf.clear();
        }
        Ok(Format {
            language: String::new(), // strings.xml does not contain language metadata
            strings: string_resources,
            plurals: plural_resources,
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

        // Write plurals
        for pr in &self.plurals {
            let mut elem = BytesStart::new("plurals");
            elem.push_attribute(("name", pr.name.as_str()));
            if let Some(trans) = pr.translatable {
                elem.push_attribute(("translatable", if trans { "true" } else { "false" }));
            }
            xml_writer.write_event(Event::Start(elem))?;
            xml_writer.write_event(Event::Text(BytesText::new("\n")))?;

            // Sort items by quantity for stable output
            let mut items = pr.items.clone();
            items.sort_by(|a, b| a.quantity.cmp(&b.quantity));
            for item in &items {
                let mut it = BytesStart::new("item");
                it.push_attribute((
                    "quantity",
                    match item.quantity {
                        PluralCategory::Zero => "zero",
                        PluralCategory::One => "one",
                        PluralCategory::Two => "two",
                        PluralCategory::Few => "few",
                        PluralCategory::Many => "many",
                        PluralCategory::Other => "other",
                    },
                ));
                xml_writer.write_event(Event::Start(it))?;
                xml_writer.write_event(Event::Text(BytesText::new(&item.value)))?;
                xml_writer.write_event(Event::End(BytesEnd::new("item")))?;
                xml_writer.write_event(Event::Text(BytesText::new("\n")))?;
            }

            xml_writer.write_event(Event::End(BytesEnd::new("plurals")))?;
            xml_writer.write_event(Event::Text(BytesText::new("\n")))?;
        }

        xml_writer.write_event(Event::End(BytesEnd::new("resources")))?;
        xml_writer.write_event(Event::Text(BytesText::new("\n")))?;
        Ok(())
    }
}

impl From<Resource> for Format {
    fn from(value: Resource) -> Self {
        let mut strings = Vec::new();
        let mut plurals = Vec::new();
        for entry in value.entries {
            match entry.value {
                Translation::Singular(_) => strings.push(StringResource::from_entry(&entry)),
                Translation::Plural(p) => {
                    let mut items: Vec<PluralItem> = p
                        .forms
                        .into_iter()
                        .map(|(cat, v)| PluralItem {
                            quantity: cat,
                            value: v,
                        })
                        .collect();
                    // Ensure stable order later
                    items.sort_by(|a, b| a.quantity.cmp(&b.quantity));
                    plurals.push(PluralsResource {
                        name: entry.id,
                        items,
                        translatable: match entry.status {
                            EntryStatus::Translated => Some(true),
                            EntryStatus::DoNotTranslate => Some(false),
                            _ => None,
                        },
                    });
                }
            }
        }

        Self {
            language: value.metadata.language,
            strings,
            plurals,
        }
    }
}

impl From<Format> for Resource {
    fn from(value: Format) -> Self {
        let mut entries: Vec<Entry> = value
            .strings
            .into_iter()
            .map(StringResource::into_entry)
            .collect();

        // Convert plurals to entries
        for pr in value.plurals {
            let mut forms = std::collections::BTreeMap::new();
            for item in pr.items {
                let PluralItem { quantity, value } = item;
                forms.insert(quantity, value);
            }
            let all_empty = forms.values().all(|v| v.is_empty());
            let status = match pr.translatable {
                Some(true) => EntryStatus::Translated,
                Some(false) => EntryStatus::DoNotTranslate,
                None => {
                    if all_empty {
                        EntryStatus::New
                    } else {
                        EntryStatus::Translated
                    }
                }
            };
            entries.push(Entry {
                id: pr.name.clone(),
                value: Translation::Plural(Plural { id: pr.name, forms }),
                comment: None,
                status,
                custom: HashMap::new(),
            });
        }

        Resource {
            metadata: Metadata {
                language: value.language,
                domain: String::new(), // strings.xml does not have a domain
                custom: HashMap::new(),
            },
            entries,
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
    fn into_entry(self) -> Entry {
        let StringResource {
            name,
            value,
            translatable,
        } = self;

        let is_value_empty = value.is_empty();

        Entry {
            id: name,
            value: Translation::Singular(value),
            comment: None,
            status: match translatable {
                Some(true) => EntryStatus::Translated,
                Some(false) => EntryStatus::DoNotTranslate,
                None if is_value_empty => EntryStatus::New,
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

#[derive(Debug, Serialize, Clone)]
pub struct PluralItem {
    pub quantity: PluralCategory,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct PluralsResource {
    pub name: String,
    pub items: Vec<PluralItem>,
    pub translatable: Option<bool>,
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
    // Read and accumulate all text nodes until we reach the end of this <string> element
    let mut value = String::new();
    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                value.push_str(e.unescape().map_err(Error::XmlParse)?.as_ref());
            }
            Ok(Event::End(ref end)) if end.name().as_ref() == b"string" => break,
            Ok(Event::Eof) => return Err(Error::InvalidResource("Unexpected EOF".to_string())),
            Ok(_) => (),
            Err(e) => return Err(Error::XmlParse(e)),
        }
        buf.clear();
    }

    // Normalize: if the content ends with a newline followed only by indentation
    // spaces, collapse that trailing indentation to 4 spaces to avoid
    // propagating XML pretty-print indentation.
    if let Some(pos) = value.rfind('\n') {
        let tail = &value[pos + 1..];
        if !tail.is_empty() && tail.chars().all(|c| c == ' ' || c == '\t') {
            value.truncate(pos + 1);
            value.push_str("    ");
        }
    }

    // Convert actual newlines into literal "\\n" sequences for internal consistency
    if value.contains('\n') {
        value = value.split('\n').collect::<Vec<_>>().join("\\n");
    }
    Ok(StringResource {
        name,
        value,
        translatable,
    })
}

fn parse_plurals_resource<R: BufRead>(
    e: &BytesStart,
    xml_reader: &mut Reader<R>,
) -> Result<PluralsResource, Error> {
    let mut name: Option<String> = None;
    let mut translatable: Option<bool> = None;

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
        name.ok_or_else(|| Error::InvalidResource("plurals tag missing 'name'".to_string()))?;

    let mut buf = Vec::new();
    let mut items: Vec<PluralItem> = Vec::new();
    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"item" => {
                // parse quantity
                let mut quantity: Option<PluralCategory> = None;
                for attr in e.attributes().with_checks(false) {
                    let attr = attr.map_err(|e| Error::DataMismatch(e.to_string()))?;
                    if attr.key.as_ref() == b"quantity" {
                        let v = attr.unescape_value()?.to_string();
                        quantity = PluralCategory::from_str(&v).ok();
                    }
                }
                let quantity = quantity
                    .ok_or_else(|| Error::InvalidResource("item missing 'quantity'".to_string()))?;
                // Read text content until End(item)
                let mut value = String::new();
                let mut local_buf = Vec::new();
                loop {
                    match xml_reader.read_event_into(&mut local_buf) {
                        Ok(Event::Text(e)) => {
                            value.push_str(e.unescape().map_err(Error::XmlParse)?.as_ref());
                        }
                        Ok(Event::End(ref end)) if end.name().as_ref() == b"item" => break,
                        Ok(Event::Eof) => {
                            return Err(Error::InvalidResource(
                                "Unexpected EOF inside <item>".to_string(),
                            ));
                        }
                        Ok(_) => {}
                        Err(e) => return Err(Error::XmlParse(e)),
                    }
                    local_buf.clear();
                }
                items.push(PluralItem { quantity, value });
            }
            Ok(Event::End(ref end)) if end.name().as_ref() == b"plurals" => break,
            Ok(Event::Eof) => {
                return Err(Error::InvalidResource(
                    "Unexpected EOF inside <plurals>".to_string(),
                ));
            }
            Ok(_) => {}
            Err(e) => return Err(Error::XmlParse(e)),
        }
        buf.clear();
    }

    Ok(PluralsResource {
        name,
        items,
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
            <string name="multiple_lines">Hello\n\n
World
            </string>
        </resources>
        "#;
        let format = Format::from_str(xml).unwrap();
        assert_eq!(format.strings.len(), 4);
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
        let multiple_lines = &format.strings[3];
        assert_eq!(multiple_lines.name, "multiple_lines");
        assert_eq!(multiple_lines.value, r#"Hello\n\n\nWorld\n    "#);
        assert_eq!(multiple_lines.translatable, None);
    }

    #[test]
    fn test_parse_plurals_included() {
        let xml = r#"
        <resources>
            <string name="hello">Hello</string>
            <plurals name="apples">
                <item quantity="one">One apple</item>
                <item quantity="other">%d apples</item>
            </plurals>
        </resources>
        "#;
        // Plurals are parsed into `plurals`
        let format = Format::from_str(xml).unwrap();
        assert_eq!(format.strings.len(), 1);
        assert_eq!(format.plurals.len(), 1);
        assert_eq!(format.strings[0].name, "hello");
        assert_eq!(format.plurals[0].name, "apples");
        assert_eq!(format.plurals[0].items.len(), 2);
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
            <plurals name="apples" translatable="true">
                <item quantity="one">One apple</item>
                <item quantity="other">%d apples</item>
            </plurals>
        </resources>
        "#;
        let format = Format::from_str(xml).unwrap();
        let mut out = Vec::new();
        format.to_writer(&mut out).unwrap();
        let out_str = String::from_utf8(out).unwrap();
        let reparsed = Format::from_str(&out_str).unwrap();
        assert_eq!(format.strings.len(), reparsed.strings.len());
        assert_eq!(format.plurals.len(), reparsed.plurals.len());
        for (orig, new) in format.strings.iter().zip(reparsed.strings.iter()) {
            assert_eq!(orig.name, new.name);
            assert_eq!(orig.value, new.value);
            assert_eq!(orig.translatable, new.translatable);
        }
        for (orig, new) in format.plurals.iter().zip(reparsed.plurals.iter()) {
            assert_eq!(orig.name, new.name);
            assert_eq!(orig.translatable, new.translatable);
            assert_eq!(orig.items.len(), new.items.len());
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
        let length = format.strings.len();
        assert_eq!(length, 1);
        let entry = format.strings.into_iter().next().unwrap().into_entry();
        assert_eq!(entry.status, EntryStatus::New);
    }

    #[test]
    fn test_resource_to_android_format_with_plurals() {
        use std::collections::BTreeMap;
        let mut forms = BTreeMap::new();
        forms.insert(PluralCategory::One, "One file".to_string());
        forms.insert(PluralCategory::Other, "%d files".to_string());

        let resource = Resource {
            metadata: Metadata {
                language: "en".into(),
                domain: String::new(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "files".into(),
                value: Translation::Plural(Plural {
                    id: "files".into(),
                    forms,
                }),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        let fmt = Format::from(resource);
        assert_eq!(fmt.strings.len(), 0);
        assert_eq!(fmt.plurals.len(), 1);
        let pr = &fmt.plurals[0];
        assert_eq!(pr.name, "files");
        assert!(
            pr.items
                .iter()
                .any(|i| matches!(i.quantity, PluralCategory::One) && i.value == "One file")
        );
        assert!(
            pr.items
                .iter()
                .any(|i| matches!(i.quantity, PluralCategory::Other) && i.value == "%d files")
        );
    }
}
