//! Support for Apple/Xcode-flavored XLIFF 1.2 localization exchange files.
//!
//! This implementation intentionally targets the narrow Xcode-style subset used
//! for localization export/import:
//! - root `<xliff version="1.2">`
//! - `<file>` groups with `source-language` and optional `target-language`
//! - `<trans-unit>` entries with plain-text `<source>`, optional `<target>`,
//!   and optional `<note>` translator comments
//!
//! More advanced XLIFF constructs such as nested inline markup, `<group>`
//! plural payloads, or XLIFF 2.0 are rejected explicitly.

use quick_xml::{
    Reader, Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    io::{BufRead, Write},
    path::Path,
};

use crate::{
    error::Error,
    traits::Parser,
    types::{Entry, EntryStatus, Metadata, Resource, Translation},
};

pub const XLIFF_ORIGINAL_KEY: &str = "xliff.original";
pub const XLIFF_DATATYPE_KEY: &str = "xliff.datatype";
pub const XLIFF_RESNAME_KEY: &str = "xliff.resname";

const DEFAULT_VERSION: &str = "1.2";
const DEFAULT_DATATYPE: &str = "plaintext";
const XLIFF_NAMESPACE: &str = "urn:oasis:names:tc:xliff:document:1.2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    pub files: Vec<FileGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileGroup {
    pub original: Option<String>,
    pub source_language: String,
    pub target_language: Option<String>,
    pub datatype: String,
    pub units: Vec<TransUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransUnit {
    pub id: String,
    pub resname: Option<String>,
    pub source: String,
    pub target: Option<String>,
    pub notes: Vec<String>,
}

impl Parser for Format {
    fn from_reader<R: BufRead>(reader: R) -> Result<Self, Error> {
        let mut xml_reader = Reader::from_reader(reader);
        xml_reader.config_mut().trim_text(false);

        let mut buf = Vec::new();
        let mut files = Vec::new();
        let mut saw_root = false;
        let mut current_file: Option<FileGroup> = None;

        loop {
            match xml_reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"xliff" => {
                    saw_root = true;
                    let version = required_attr(e, b"version", "<xliff>")?;
                    if version != DEFAULT_VERSION {
                        return Err(Error::UnsupportedFormat(format!(
                            "Unsupported XLIFF version '{}'. Only XLIFF 1.2 is supported.",
                            version
                        )));
                    }
                }
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"file" => {
                    if current_file.is_some() {
                        return Err(Error::InvalidResource(
                            "Nested <file> elements are not supported".to_string(),
                        ));
                    }
                    current_file = Some(parse_file_group_start(e)?);
                }
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"body" => {}
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"trans-unit" => {
                    let file = current_file.as_mut().ok_or_else(|| {
                        Error::InvalidResource(
                            "<trans-unit> encountered outside of a <file> group".to_string(),
                        )
                    })?;
                    let unit = parse_trans_unit(e, &mut xml_reader, &file.target_language)?;
                    file.units.push(unit);
                }
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"group" => {
                    return Err(Error::UnsupportedFormat(
                        "Plural/group XLIFF payloads are not supported in v1".to_string(),
                    ));
                }
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"bin-unit" => {
                    return Err(Error::UnsupportedFormat(
                        "Binary XLIFF units are not supported in v1".to_string(),
                    ));
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"file" => {
                    let file = current_file.take().ok_or_else(|| {
                        Error::InvalidResource("Unexpected </file> without matching <file>".into())
                    })?;
                    validate_file_group(&file)?;
                    files.push(file);
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => return Err(Error::XmlParse(e)),
            }
            buf.clear();
        }

        if !saw_root {
            return Err(Error::InvalidResource(
                "Missing <xliff> root element".to_string(),
            ));
        }

        if current_file.is_some() {
            return Err(Error::InvalidResource(
                "Unexpected EOF before closing </file>".to_string(),
            ));
        }

        Ok(Self { files })
    }

    fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        let mut xml_writer = Writer::new(&mut writer);

        xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
        xml_writer.write_event(Event::Text(BytesText::new("\n")))?;

        let mut root = BytesStart::new("xliff");
        root.push_attribute(("xmlns", XLIFF_NAMESPACE));
        root.push_attribute(("version", DEFAULT_VERSION));
        xml_writer.write_event(Event::Start(root))?;
        xml_writer.write_event(Event::Text(BytesText::new("\n")))?;

        let mut files = self.files.clone();
        files.sort_by(|a, b| {
            let left = a.original.as_deref().unwrap_or("");
            let right = b.original.as_deref().unwrap_or("");
            left.cmp(right)
                .then_with(|| a.source_language.cmp(&b.source_language))
                .then_with(|| a.target_language.cmp(&b.target_language))
        });

        for file in &files {
            write_indent(&mut xml_writer, 1)?;
            let mut file_start = BytesStart::new("file");
            file_start.push_attribute(("source-language", file.source_language.as_str()));
            if let Some(target_language) = &file.target_language {
                file_start.push_attribute(("target-language", target_language.as_str()));
            }
            if let Some(original) = &file.original {
                file_start.push_attribute(("original", original.as_str()));
            }
            file_start.push_attribute(("datatype", file.datatype.as_str()));
            xml_writer.write_event(Event::Start(file_start))?;
            xml_writer.write_event(Event::Text(BytesText::new("\n")))?;

            write_indent(&mut xml_writer, 2)?;
            xml_writer.write_event(Event::Start(BytesStart::new("body")))?;
            xml_writer.write_event(Event::Text(BytesText::new("\n")))?;

            let mut units = file.units.clone();
            units.sort_by(|a, b| a.id.cmp(&b.id));
            for unit in &units {
                write_indent(&mut xml_writer, 3)?;
                let mut unit_start = BytesStart::new("trans-unit");
                unit_start.push_attribute(("id", unit.id.as_str()));
                unit_start.push_attribute(("xml:space", "preserve"));
                if let Some(resname) = &unit.resname {
                    unit_start.push_attribute(("resname", resname.as_str()));
                }
                xml_writer.write_event(Event::Start(unit_start))?;
                xml_writer.write_event(Event::Text(BytesText::new("\n")))?;

                write_text_element(&mut xml_writer, 4, "source", &unit.source)?;
                if file.target_language.is_some() {
                    write_optional_text_element(
                        &mut xml_writer,
                        4,
                        "target",
                        unit.target.as_deref(),
                    )?;
                }
                for note in &unit.notes {
                    write_text_element(&mut xml_writer, 4, "note", note)?;
                }

                write_indent(&mut xml_writer, 3)?;
                xml_writer.write_event(Event::End(BytesEnd::new("trans-unit")))?;
                xml_writer.write_event(Event::Text(BytesText::new("\n")))?;
            }

            write_indent(&mut xml_writer, 2)?;
            xml_writer.write_event(Event::End(BytesEnd::new("body")))?;
            xml_writer.write_event(Event::Text(BytesText::new("\n")))?;

            write_indent(&mut xml_writer, 1)?;
            xml_writer.write_event(Event::End(BytesEnd::new("file")))?;
            xml_writer.write_event(Event::Text(BytesText::new("\n")))?;
        }

        xml_writer.write_event(Event::End(BytesEnd::new("xliff")))?;
        xml_writer.write_event(Event::Text(BytesText::new("\n")))?;
        Ok(())
    }
}

impl Format {
    pub fn from_resources(
        resources: Vec<Resource>,
        source_language_hint: Option<&str>,
        target_language_hint: Option<&str>,
    ) -> Result<Self, Error> {
        if resources.is_empty() {
            return Err(Error::InvalidResource(
                "No resources provided for XLIFF output".to_string(),
            ));
        }

        let languages = collect_languages(&resources)?;
        let source_language = resolve_source_language(
            &resources,
            &languages,
            source_language_hint,
            target_language_hint,
        )?;
        let target_language =
            resolve_target_language(&languages, &source_language, target_language_hint)?;

        let mut group_map: BTreeMap<String, GroupAccumulator> = BTreeMap::new();

        for resource in resources {
            if resource.metadata.language != source_language
                && resource.metadata.language != target_language
            {
                return Err(Error::InvalidResource(format!(
                    "XLIFF output requires exactly one source language ('{}') and one target language ('{}'), but found extra language '{}'",
                    source_language, target_language, resource.metadata.language
                )));
            }

            let group_key = resource
                .metadata
                .custom
                .get(XLIFF_ORIGINAL_KEY)
                .cloned()
                .or_else(|| {
                    let trimmed = resource.metadata.domain.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .unwrap_or_else(|| "Localizable".to_string());

            let datatype = resource
                .metadata
                .custom
                .get(XLIFF_DATATYPE_KEY)
                .cloned()
                .unwrap_or_else(|| DEFAULT_DATATYPE.to_string());

            reject_plural_like_metadata(
                resource
                    .metadata
                    .custom
                    .get(XLIFF_ORIGINAL_KEY)
                    .map(String::as_str),
                Some(datatype.as_str()),
            )?;

            let group = group_map
                .entry(group_key.clone())
                .or_insert_with(|| GroupAccumulator {
                    original: Some(group_key.clone()),
                    datatype: datatype.clone(),
                    source_entries: BTreeMap::new(),
                    target_entries: BTreeMap::new(),
                });

            if group.datatype != datatype {
                return Err(Error::DataMismatch(format!(
                    "Conflicting XLIFF datatype metadata for group '{}': '{}' vs '{}'",
                    group_key, group.datatype, datatype
                )));
            }

            for entry in resource.entries {
                let prepared = PreparedEntry::from_entry(entry)?;
                let prepared_id = prepared.id.clone();
                let destination = if resource.metadata.language == source_language {
                    &mut group.source_entries
                } else {
                    &mut group.target_entries
                };

                if destination.insert(prepared_id.clone(), prepared).is_some() {
                    return Err(Error::InvalidResource(format!(
                        "Duplicate XLIFF entry id '{}' in group '{}' for language '{}'",
                        prepared_id, group_key, resource.metadata.language
                    )));
                }
            }
        }

        let mut files = Vec::new();
        for group in group_map.into_values() {
            let mut units = Vec::new();
            for (id, source_entry) in group.source_entries {
                let target_entry = group.target_entries.get(&id);
                let comment = source_entry
                    .comment
                    .clone()
                    .or_else(|| target_entry.and_then(|entry| entry.comment.clone()));
                let target = target_entry.map(|entry| entry.value.clone());

                units.push(TransUnit {
                    id,
                    resname: source_entry.resname.clone(),
                    source: source_entry.value,
                    target,
                    notes: split_comment_into_notes(comment.as_deref()),
                });
            }

            for extra_target_id in group.target_entries.keys() {
                if !units.iter().any(|unit| &unit.id == extra_target_id) {
                    return Err(Error::InvalidResource(format!(
                        "Target XLIFF entry '{}' is missing a matching source entry in group '{}'",
                        extra_target_id,
                        group.original.as_deref().unwrap_or(""),
                    )));
                }
            }

            files.push(FileGroup {
                original: group.original,
                source_language: source_language.clone(),
                target_language: Some(target_language.clone()),
                datatype: group.datatype,
                units,
            });
        }

        Ok(Self { files })
    }
}

impl TryFrom<Vec<Resource>> for Format {
    type Error = Error;

    fn try_from(resources: Vec<Resource>) -> Result<Self, Self::Error> {
        Self::from_resources(resources, None, None)
    }
}

impl TryFrom<Format> for Vec<Resource> {
    type Error = Error;

    fn try_from(format: Format) -> Result<Self, Self::Error> {
        let mut resources = Vec::new();

        for file in format.files {
            validate_file_group(&file)?;

            let domain = domain_from_original(file.original.as_deref());
            let mut base_custom = HashMap::new();
            base_custom.insert("source_language".to_string(), file.source_language.clone());
            if let Some(original) = &file.original {
                base_custom.insert(XLIFF_ORIGINAL_KEY.to_string(), original.clone());
            }
            base_custom.insert(XLIFF_DATATYPE_KEY.to_string(), file.datatype.clone());

            let mut source_entries = Vec::new();
            let mut target_entries = Vec::new();
            let mut seen_ids = HashSet::new();

            for unit in file.units {
                if !seen_ids.insert(unit.id.clone()) {
                    return Err(Error::InvalidResource(format!(
                        "Duplicate trans-unit id '{}' within XLIFF file group '{}'",
                        unit.id,
                        file.original.as_deref().unwrap_or(""),
                    )));
                }

                let comment = notes_to_comment(&unit.notes);

                let mut source_custom = HashMap::new();
                if let Some(resname) = unit.resname.as_ref().filter(|resname| *resname != &unit.id)
                {
                    source_custom.insert(XLIFF_RESNAME_KEY.to_string(), resname.clone());
                }

                source_entries.push(Entry {
                    id: unit.id.clone(),
                    value: Translation::Singular(unit.source),
                    comment: comment.clone(),
                    status: EntryStatus::Translated,
                    custom: source_custom.clone(),
                });

                if let Some(target_language) = &file.target_language {
                    let target_value = unit.target.unwrap_or_default();
                    let has_target_value = !target_value.is_empty();
                    target_entries.push(Entry {
                        id: unit.id,
                        value: if has_target_value {
                            Translation::Singular(target_value)
                        } else {
                            Translation::Empty
                        },
                        comment,
                        status: if has_target_value {
                            EntryStatus::Translated
                        } else {
                            EntryStatus::New
                        },
                        custom: source_custom,
                    });

                    if target_language.trim().is_empty() {
                        return Err(Error::InvalidResource(
                            "XLIFF target-language attribute cannot be empty".to_string(),
                        ));
                    }
                }
            }

            resources.push(Resource {
                metadata: Metadata {
                    language: file.source_language.clone(),
                    domain: domain.clone(),
                    custom: base_custom.clone(),
                },
                entries: source_entries,
            });

            if let Some(target_language) = file.target_language {
                resources.push(Resource {
                    metadata: Metadata {
                        language: target_language,
                        domain,
                        custom: base_custom,
                    },
                    entries: target_entries,
                });
            }
        }

        Ok(resources)
    }
}

#[derive(Debug, Clone)]
struct PreparedEntry {
    id: String,
    value: String,
    comment: Option<String>,
    resname: Option<String>,
}

impl PreparedEntry {
    fn from_entry(entry: Entry) -> Result<Self, Error> {
        let value = match entry.value {
            Translation::Empty => String::new(),
            Translation::Singular(value) => value,
            Translation::Plural(_) => {
                return Err(Error::UnsupportedFormat(format!(
                    "Plural entry '{}' cannot be represented in XLIFF v1 output",
                    entry.id
                )));
            }
        };

        let resname = entry
            .custom
            .get(XLIFF_RESNAME_KEY)
            .cloned()
            .filter(|resname| resname != &entry.id);

        Ok(Self {
            id: entry.id,
            value,
            comment: entry.comment,
            resname,
        })
    }
}

#[derive(Debug, Default)]
struct GroupAccumulator {
    original: Option<String>,
    datatype: String,
    source_entries: BTreeMap<String, PreparedEntry>,
    target_entries: BTreeMap<String, PreparedEntry>,
}

fn parse_file_group_start(e: &BytesStart<'_>) -> Result<FileGroup, Error> {
    let source_language = required_attr(e, b"source-language", "<file>")?;
    let target_language = optional_attr(e, b"target-language")?;
    let original = optional_attr(e, b"original")?;
    let datatype = optional_attr(e, b"datatype")?.unwrap_or_else(|| DEFAULT_DATATYPE.to_string());

    reject_plural_like_metadata(original.as_deref(), Some(datatype.as_str()))?;

    Ok(FileGroup {
        original,
        source_language,
        target_language,
        datatype,
        units: Vec::new(),
    })
}

fn parse_trans_unit<R: BufRead>(
    e: &BytesStart<'_>,
    xml_reader: &mut Reader<R>,
    target_language: &Option<String>,
) -> Result<TransUnit, Error> {
    let id = required_attr(e, b"id", "<trans-unit>")?;
    let resname = optional_attr(e, b"resname")?;

    let mut buf = Vec::new();
    let mut source = None;
    let mut target = None;
    let mut notes = Vec::new();

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref child)) if child.name().as_ref() == b"source" => {
                source = Some(read_plain_text_element(xml_reader, b"source")?);
            }
            Ok(Event::Empty(ref child)) if child.name().as_ref() == b"source" => {
                source = Some(String::new());
            }
            Ok(Event::Start(ref child)) if child.name().as_ref() == b"target" => {
                if target_language.is_none() {
                    return Err(Error::InvalidResource(format!(
                        "Found <target> for trans-unit '{}' but the enclosing <file> is missing target-language",
                        id
                    )));
                }
                target = Some(read_plain_text_element(xml_reader, b"target")?);
            }
            Ok(Event::Empty(ref child)) if child.name().as_ref() == b"target" => {
                if target_language.is_none() {
                    return Err(Error::InvalidResource(format!(
                        "Found <target> for trans-unit '{}' but the enclosing <file> is missing target-language",
                        id
                    )));
                }
                target = Some(String::new());
            }
            Ok(Event::Start(ref child)) if child.name().as_ref() == b"note" => {
                notes.push(read_plain_text_element(xml_reader, b"note")?);
            }
            Ok(Event::Empty(ref child)) if child.name().as_ref() == b"note" => {
                notes.push(String::new());
            }
            Ok(Event::Start(ref child)) if child.name().as_ref() == b"group" => {
                return Err(Error::UnsupportedFormat(format!(
                    "Plural/group XLIFF payloads are not supported in trans-unit '{}'",
                    id
                )));
            }
            Ok(Event::End(ref child)) if child.name().as_ref() == b"trans-unit" => break,
            Ok(Event::Eof) => {
                return Err(Error::InvalidResource(format!(
                    "Unexpected EOF inside trans-unit '{}'",
                    id
                )));
            }
            Ok(_) => {}
            Err(e) => return Err(Error::XmlParse(e)),
        }
        buf.clear();
    }

    let source = source.ok_or_else(|| {
        Error::InvalidResource(format!(
            "trans-unit '{}' is missing a required <source> element",
            id
        ))
    })?;

    Ok(TransUnit {
        id,
        resname,
        source,
        target,
        notes,
    })
}

fn read_plain_text_element<R: BufRead>(
    xml_reader: &mut Reader<R>,
    element_name: &[u8],
) -> Result<String, Error> {
    let mut buf = Vec::new();
    let mut text = String::new();

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                text.push_str(e.unescape().map_err(Error::XmlParse)?.as_ref());
            }
            Ok(Event::CData(e)) => {
                let cdata = std::str::from_utf8(e.as_ref()).map_err(|_| {
                    Error::InvalidResource(format!(
                        "Invalid UTF-8 inside <{}> CDATA section",
                        String::from_utf8_lossy(element_name)
                    ))
                })?;
                text.push_str(cdata);
            }
            Ok(Event::Start(_)) | Ok(Event::Empty(_)) => {
                return Err(Error::UnsupportedFormat(format!(
                    "Inline markup inside <{}> is not supported in XLIFF v1",
                    String::from_utf8_lossy(element_name)
                )));
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == element_name => break,
            Ok(Event::Eof) => {
                return Err(Error::InvalidResource(format!(
                    "Unexpected EOF inside <{}>",
                    String::from_utf8_lossy(element_name)
                )));
            }
            Ok(_) => {}
            Err(e) => return Err(Error::XmlParse(e)),
        }
        buf.clear();
    }

    Ok(text)
}

fn validate_file_group(file: &FileGroup) -> Result<(), Error> {
    if file.source_language.trim().is_empty() {
        return Err(Error::InvalidResource(
            "XLIFF file group is missing source-language metadata".to_string(),
        ));
    }
    if let Some(target_language) = &file.target_language
        && target_language.trim().is_empty()
    {
        return Err(Error::InvalidResource(
            "XLIFF target-language attribute cannot be empty".to_string(),
        ));
    }

    reject_plural_like_metadata(file.original.as_deref(), Some(file.datatype.as_str()))?;

    let mut seen_ids = HashSet::new();
    for unit in &file.units {
        if !seen_ids.insert(unit.id.clone()) {
            return Err(Error::InvalidResource(format!(
                "Duplicate trans-unit id '{}' within XLIFF file group '{}'",
                unit.id,
                file.original.as_deref().unwrap_or(""),
            )));
        }
    }

    Ok(())
}

fn collect_languages(resources: &[Resource]) -> Result<BTreeSet<String>, Error> {
    let mut languages = BTreeSet::new();
    for resource in resources {
        let language = resource.metadata.language.trim();
        if language.is_empty() {
            return Err(Error::InvalidResource(
                "XLIFF output requires every resource to have a language".to_string(),
            ));
        }
        languages.insert(language.to_string());
    }
    Ok(languages)
}

fn resolve_source_language(
    resources: &[Resource],
    languages: &BTreeSet<String>,
    source_language_hint: Option<&str>,
    target_language_hint: Option<&str>,
) -> Result<String, Error> {
    if let Some(source_language_hint) = source_language_hint {
        let source_language = source_language_hint.trim();
        if source_language.is_empty() {
            return Err(Error::InvalidResource(
                "--source-language cannot be empty for XLIFF output".to_string(),
            ));
        }
        if !languages.contains(source_language) {
            return Err(Error::InvalidResource(format!(
                "XLIFF source language '{}' was requested, but available resource languages are: {}",
                source_language,
                languages.iter().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        return Ok(source_language.to_string());
    }

    let metadata_source_languages = resources
        .iter()
        .filter_map(|resource| resource.metadata.custom.get("source_language"))
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect::<BTreeSet<_>>();

    if metadata_source_languages.len() == 1 {
        let source_language = metadata_source_languages.iter().next().cloned().unwrap();
        if !languages.contains(&source_language) {
            return Err(Error::InvalidResource(format!(
                "XLIFF source language metadata '{}' does not match any available resource languages ({})",
                source_language,
                languages.iter().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        return Ok(source_language);
    }

    if let Some(target_language_hint) = target_language_hint {
        let candidates = languages
            .iter()
            .filter(|language| language.as_str() != target_language_hint)
            .cloned()
            .collect::<Vec<_>>();

        if candidates.len() == 1 {
            return Ok(candidates[0].clone());
        }
    }

    Err(Error::InvalidResource(
        "XLIFF output requires a resolvable source language. Pass --source-language or provide consistent source_language metadata.".to_string(),
    ))
}

fn resolve_target_language(
    languages: &BTreeSet<String>,
    source_language: &str,
    target_language_hint: Option<&str>,
) -> Result<String, Error> {
    if let Some(target_language_hint) = target_language_hint {
        let target_language = target_language_hint.trim();
        if target_language.is_empty() {
            return Err(Error::InvalidResource(
                "--output-lang cannot be empty for XLIFF output".to_string(),
            ));
        }
        if target_language == source_language {
            return Err(Error::InvalidResource(format!(
                "XLIFF target language '{}' must differ from source language '{}'",
                target_language, source_language
            )));
        }

        let extras = languages
            .iter()
            .filter(|language| {
                language.as_str() != source_language && language.as_str() != target_language
            })
            .cloned()
            .collect::<Vec<_>>();
        if !extras.is_empty() {
            return Err(Error::InvalidResource(format!(
                "XLIFF output requires exactly one target language. Extra languages present: {}",
                extras.join(", ")
            )));
        }

        return Ok(target_language.to_string());
    }

    let targets = languages
        .iter()
        .filter(|language| language.as_str() != source_language)
        .cloned()
        .collect::<Vec<_>>();

    match targets.as_slice() {
        [target_language] => Ok(target_language.clone()),
        [] => Err(Error::InvalidResource(
            "XLIFF output requires a target language. Pass --output-lang and include the target resource.".to_string(),
        )),
        _ => Err(Error::InvalidResource(format!(
            "XLIFF output requires exactly one target language, but found: {}. Pass --output-lang.",
            targets.join(", ")
        ))),
    }
}

fn required_attr(e: &BytesStart<'_>, attr_name: &[u8], element: &str) -> Result<String, Error> {
    optional_attr(e, attr_name)?.ok_or_else(|| {
        Error::InvalidResource(format!(
            "{} is missing required attribute '{}'",
            element,
            String::from_utf8_lossy(attr_name)
        ))
    })
}

fn optional_attr(e: &BytesStart<'_>, attr_name: &[u8]) -> Result<Option<String>, Error> {
    for attr in e.attributes().with_checks(false) {
        let attr = attr.map_err(|e| Error::DataMismatch(e.to_string()))?;
        if attr.key.as_ref() == attr_name {
            return Ok(Some(attr.unescape_value()?.to_string()));
        }
    }
    Ok(None)
}

fn reject_plural_like_metadata(
    original: Option<&str>,
    datatype: Option<&str>,
) -> Result<(), Error> {
    let looks_plural = |value: &str| {
        let lower = value.to_ascii_lowercase();
        lower.contains("stringsdict") || lower.contains("plural")
    };

    if let Some(original) = original
        && looks_plural(original)
    {
        return Err(Error::UnsupportedFormat(format!(
            "Plural-like XLIFF source '{}' is not supported in v1",
            original
        )));
    }
    if let Some(datatype) = datatype
        && looks_plural(datatype)
    {
        return Err(Error::UnsupportedFormat(format!(
            "Plural-like XLIFF datatype '{}' is not supported in v1",
            datatype
        )));
    }
    Ok(())
}

fn domain_from_original(original: Option<&str>) -> String {
    original
        .and_then(|original| {
            Path::new(original)
                .file_name()
                .and_then(|file| Path::new(file).file_stem())
        })
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string()
}

fn notes_to_comment(notes: &[String]) -> Option<String> {
    let notes = notes
        .iter()
        .map(|note| note.trim().to_string())
        .filter(|note| !note.is_empty())
        .collect::<Vec<_>>();
    if notes.is_empty() {
        None
    } else {
        Some(notes.join("\n\n"))
    }
}

fn split_comment_into_notes(comment: Option<&str>) -> Vec<String> {
    let Some(comment) = comment else {
        return Vec::new();
    };

    let normalized = comment.replace("\r\n", "\n");
    normalized
        .split("\n\n")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
        .collect()
}

fn write_indent<W: Write>(writer: &mut Writer<W>, depth: usize) -> Result<(), Error> {
    let indent = "  ".repeat(depth);
    writer.write_event(Event::Text(BytesText::new(&indent)))?;
    Ok(())
}

fn write_text_element<W: Write>(
    writer: &mut Writer<W>,
    depth: usize,
    name: &str,
    value: &str,
) -> Result<(), Error> {
    write_indent(writer, depth)?;
    writer.write_event(Event::Start(BytesStart::new(name)))?;
    if !value.is_empty() {
        writer.write_event(Event::Text(BytesText::new(value)))?;
    }
    writer.write_event(Event::End(BytesEnd::new(name)))?;
    writer.write_event(Event::Text(BytesText::new("\n")))?;
    Ok(())
}

fn write_optional_text_element<W: Write>(
    writer: &mut Writer<W>,
    depth: usize,
    name: &str,
    value: Option<&str>,
) -> Result<(), Error> {
    if let Some(value) = value {
        write_text_element(writer, depth, name, value)
    } else {
        write_indent(writer, depth)?;
        writer.write_event(Event::Empty(BytesStart::new(name)))?;
        writer.write_event(Event::Text(BytesText::new("\n")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_xliff(input: &str) -> Format {
        Format::from_str(input).unwrap()
    }

    #[test]
    fn parses_bilingual_xcode_xliff_with_notes_and_missing_target() {
        let xliff = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="Base.lproj/Localizable.strings" source-language="en" target-language="fr" datatype="plaintext">
    <body>
      <trans-unit id="greeting" resname="GREETING" xml:space="preserve">
        <source>Hello</source>
        <target>Bonjour</target>
        <note>Shown on the home screen.</note>
        <note>Keep it short.</note>
      </trans-unit>
      <trans-unit id="pending" xml:space="preserve">
        <source>Pending</source>
      </trans-unit>
    </body>
  </file>
</xliff>
"#;

        let resources = Vec::<Resource>::try_from(parse_xliff(xliff)).unwrap();
        assert_eq!(resources.len(), 2);

        let source = resources
            .iter()
            .find(|r| r.metadata.language == "en")
            .unwrap();
        let target = resources
            .iter()
            .find(|r| r.metadata.language == "fr")
            .unwrap();

        assert_eq!(source.metadata.domain, "Localizable");
        assert_eq!(
            source
                .metadata
                .custom
                .get(XLIFF_ORIGINAL_KEY)
                .map(String::as_str),
            Some("Base.lproj/Localizable.strings")
        );
        assert_eq!(
            source.find_entry("greeting").unwrap().comment.as_deref(),
            Some("Shown on the home screen.\n\nKeep it short.")
        );
        assert_eq!(
            source
                .find_entry("greeting")
                .unwrap()
                .custom
                .get(XLIFF_RESNAME_KEY)
                .map(String::as_str),
            Some("GREETING")
        );
        assert_eq!(
            target.find_entry("pending").unwrap().value,
            Translation::Empty
        );
        assert_eq!(
            target.find_entry("pending").unwrap().status,
            EntryStatus::New
        );
    }

    #[test]
    fn rejects_missing_trans_unit_id() {
        let xliff = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="Base.lproj/Localizable.strings" source-language="en" target-language="fr">
    <body>
      <trans-unit>
        <source>Hello</source>
      </trans-unit>
    </body>
  </file>
</xliff>
"#;

        let err = Format::from_str(xliff).unwrap_err();
        assert!(err.to_string().contains("trans-unit"));
        assert!(err.to_string().contains("id"));
    }

    #[test]
    fn rejects_duplicate_ids_within_file_group() {
        let xliff = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="Base.lproj/Localizable.strings" source-language="en" target-language="fr">
    <body>
      <trans-unit id="hello"><source>Hello</source></trans-unit>
      <trans-unit id="hello"><source>Hi</source></trans-unit>
    </body>
  </file>
</xliff>
"#;

        let err = Format::from_str(xliff).unwrap_err();
        assert!(err.to_string().contains("Duplicate trans-unit id"));
    }

    #[test]
    fn rejects_missing_target_language_when_target_elements_exist() {
        let xliff = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="Base.lproj/Localizable.strings" source-language="en">
    <body>
      <trans-unit id="hello">
        <source>Hello</source>
        <target>Bonjour</target>
      </trans-unit>
    </body>
  </file>
</xliff>
"#;

        let err = Format::from_str(xliff).unwrap_err();
        assert!(err.to_string().contains("target-language"));
    }

    #[test]
    fn rejects_non_12_version() {
        let xliff = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff version="2.0" xmlns="urn:oasis:names:tc:xliff:document:2.0"></xliff>
"#;

        let err = Format::from_str(xliff).unwrap_err();
        assert!(err.to_string().contains("Only XLIFF 1.2 is supported"));
    }

    #[test]
    fn rejects_plural_like_payloads() {
        let xliff = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="Base.lproj/Localizable.stringsdict" source-language="en" target-language="fr">
    <body>
      <trans-unit id="hello"><source>Hello</source></trans-unit>
    </body>
  </file>
</xliff>
"#;

        let err = Format::from_str(xliff).unwrap_err();
        assert!(err.to_string().contains("not supported"));
    }

    #[test]
    fn writes_stable_bilingual_xliff() {
        let mut source_custom = HashMap::new();
        source_custom.insert("source_language".to_string(), "en".to_string());
        source_custom.insert(
            XLIFF_ORIGINAL_KEY.to_string(),
            "Base.lproj/Localizable.strings".to_string(),
        );
        source_custom.insert(XLIFF_DATATYPE_KEY.to_string(), "plaintext".to_string());

        let source = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "Localizable".to_string(),
                custom: source_custom.clone(),
            },
            entries: vec![Entry {
                id: "greeting".to_string(),
                value: Translation::Singular("Hello".to_string()),
                comment: Some("Shown on the home screen.\n\nKeep it short.".to_string()),
                status: EntryStatus::Translated,
                custom: HashMap::from([(XLIFF_RESNAME_KEY.to_string(), "GREETING".to_string())]),
            }],
        };

        let target = Resource {
            metadata: Metadata {
                language: "fr".to_string(),
                domain: "Localizable".to_string(),
                custom: source_custom,
            },
            entries: vec![Entry {
                id: "greeting".to_string(),
                value: Translation::Singular("Bonjour".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        let format = Format::from_resources(vec![source, target], Some("en"), Some("fr")).unwrap();
        let mut out = Vec::new();
        format.to_writer(&mut out).unwrap();
        let xml = String::from_utf8(out).unwrap();

        assert!(xml.contains(r#"<file source-language="en" target-language="fr" original="Base.lproj/Localizable.strings" datatype="plaintext">"#));
        assert!(
            xml.contains(r#"<trans-unit id="greeting" xml:space="preserve" resname="GREETING">"#)
        );
        assert!(xml.contains("<note>Shown on the home screen.</note>"));
        assert!(xml.contains("<note>Keep it short.</note>"));
        assert!(xml.contains("<target>Bonjour</target>"));
    }

    #[test]
    fn infers_source_and_target_for_round_trip_output() {
        let resources = vec![
            Resource {
                metadata: Metadata {
                    language: "en".to_string(),
                    domain: "Localizable".to_string(),
                    custom: HashMap::new(),
                },
                entries: vec![Entry {
                    id: "hello".to_string(),
                    value: Translation::Singular("Hello".to_string()),
                    comment: None,
                    status: EntryStatus::Translated,
                    custom: HashMap::new(),
                }],
            },
            Resource {
                metadata: Metadata {
                    language: "fr".to_string(),
                    domain: "Localizable".to_string(),
                    custom: HashMap::from([("source_language".to_string(), "en".to_string())]),
                },
                entries: vec![Entry {
                    id: "hello".to_string(),
                    value: Translation::Singular("Bonjour".to_string()),
                    comment: None,
                    status: EntryStatus::Translated,
                    custom: HashMap::new(),
                }],
            },
        ];

        let format = Format::try_from(resources).unwrap();
        assert_eq!(format.files.len(), 1);
        assert_eq!(format.files[0].source_language, "en");
        assert_eq!(format.files[0].target_language.as_deref(), Some("fr"));
    }

    #[test]
    fn allows_explicit_target_language_without_existing_target_resource() {
        let resources = vec![Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "Localizable".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "hello".to_string(),
                value: Translation::Singular("Hello".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        }];

        let format = Format::from_resources(resources, Some("en"), Some("fr")).unwrap();
        assert_eq!(format.files[0].target_language.as_deref(), Some("fr"));
        assert_eq!(format.files[0].units[0].target, None);
    }
}
