//! Support for Apple `.strings` localization format.
//!
//! Provides parsing, serialization, and conversion to/from the internal `Resource` model.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use indoc::indoc;

use crate::{
    error::Error,
    traits::Parser,
    types::{Entry, EntryStatus, Metadata, Resource, Translation},
};

/// Represents an Apple `.strings` localization file.
///
/// The format consists of a set of key-value pairs, with optional comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    /// Language code for this resource, if known (typically empty for `.strings`).
    pub language: String,
    /// All key-value pairs (and optional comments) in the file.
    pub pairs: Vec<Pair>,
}

impl Format {
    pub fn multiline_values_to_one_line(content: &mut String) {
        // Copy of content to operate on
        let orig = content.clone();
        let mut result = String::with_capacity(orig.len());

        // State for parsing
        let mut chars = orig.chars().peekable();
        let mut inside_value = false;
        let mut value_buf = String::new();

        while let Some(c) = chars.next() {
            if !inside_value {
                // Look for start of a value: " = "
                result.push(c);
                if c == '=' {
                    // Seek first quote after '='
                    while let Some(&d) = chars.peek() {
                        result.push(d);
                        chars.next();
                        if d == '"' {
                            inside_value = true;
                            value_buf.clear();
                            break;
                        }
                    }
                }
            } else {
                // Inside value (quoted string)
                if c == '"' {
                    // Check if this quote is escaped
                    let prev_backslashes =
                        value_buf.chars().rev().take_while(|&x| x == '\\').count();
                    if prev_backslashes % 2 == 0 {
                        // Closing quote (not escaped)
                        inside_value = false;
                        // Replace newlines with \n, remove leading spaces after each
                        let value_one_line = value_buf
                            .lines()
                            .map(str::trim_start)
                            .collect::<Vec<_>>()
                            .join(r"\n");
                        result.push_str(&value_one_line);
                        result.push('"');
                        value_buf.clear();
                    } else {
                        value_buf.push('"');
                    }
                } else {
                    value_buf.push(c);
                }
            }
        }

        *content = result;
    }
}

impl Parser for Format {
    /// Creates a new `Format` instance with the specified language and pairs.
    ///
    /// The `language` parameter would be empty, since the .strings format does
    /// not contain any metadata about the language.
    fn from_reader<R: std::io::BufRead>(reader: R) -> Result<Self, Error> {
        let mut file_content = reader.lines().collect::<Result<Vec<_>, _>>()?.join("\n");

        Format::multiline_values_to_one_line(&mut file_content);

        // For simplicity, we assume there are no multi-line comments and in-line comments in the file.
        let lines = file_content.lines().collect::<Vec<_>>();

        let mut header = HashMap::<String, String>::new();

        let mut last_comment: Option<&str> = None;

        // strings pair pattern: "key" = "value";
        let pairs = lines
            .iter()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with("//:") {
                    // This is a header line, we can extract metadata from it.
                    //
                    // Example: "//: Language: English"
                    let parts: Vec<&str> = trimmed.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let key = parts[1].trim().to_string();
                        let value = parts[2].trim().to_string();
                        header.insert(key, value);
                    }
                    return None; // Skip header lines
                } else if trimmed.is_empty()
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with("//")
                {
                    last_comment = Some(trimmed);
                    return None; // Skip empty lines and comments
                }

                let parts: Vec<&str> = trimmed.splitn(3, '=').collect();
                if parts.len() != 2 {
                    return None; // Invalid line format
                }

                let key = parts[0].trim().trim_matches('"').to_string();
                let mut value = parts[1].trim().trim_matches(';').trim().to_string();

                if value.len() < 2 {
                    value = String::new(); // If value is too short, treat it as empty
                } else {
                    value = value[1..value.len() - 1].to_string(); // Remove surrounding quotes
                }

                let comment = match last_comment {
                    Some(comment) if comment.starts_with("/*") || comment.starts_with("//") => {
                        Some(comment.trim().to_string())
                    }
                    _ => None,
                };

                // Clear the last_comment after using it
                if comment.is_some() {
                    last_comment = None;
                }

                Some(Pair {
                    key,
                    value,
                    comment,
                })
            })
            .collect();

        // Extract language from header if available
        let language = &header.get("Language").cloned().unwrap_or_default();

        Ok(Format {
            language: language.to_string(), // .strings format does not have a language field
            pairs,
        })
    }

    fn to_writer<W: std::io::Write>(&self, mut writer: W) -> Result<(), Error> {
        let mut content = String::new();

        let header = format!(
            indoc! {"
            // This file is automatically generated by langcodec.
            // Do not edit it manually, as your changes will be overwritten.
            // Here's the basic information about the file which could be useful
            // for translators, and langcodec would use it to generate the
            // appropriate metadata for the resource.
            //
            //: Language: {}
            //

            "},
            self.language
        );

        content.push_str(&header);

        for pair in &self.pairs {
            content.push_str(&pair.to_string());
            content.push('\n');
        }

        writer.write_all(content.as_bytes()).map_err(Error::Io)
    }

    /// Override default file reading to support BOM-aware decoding (e.g., UTF-16 Apple .strings)
    fn read_from<P: AsRef<Path>>(path: P) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let file = File::open(path).map_err(Error::Io)?;
        // Auto-detect BOM, decode to UTF-8; passthrough UTF-8
        let mut decoder = encoding_rs_io::DecodeReaderBytesBuilder::new()
            .bom_override(true)
            .build(file);

        let mut decoded = String::new();
        decoder.read_to_string(&mut decoded).map_err(Error::Io)?;

        Self::from_str(&decoded)
    }
}

impl From<Format> for Resource {
    fn from(value: Format) -> Self {
        Resource {
            metadata: Metadata {
                language: value.language,
                domain: String::from(""),
                custom: HashMap::new(),
            },
            entries: value.pairs.iter().map(Pair::to_entry).collect(),
        }
    }
}

impl TryFrom<Resource> for Format {
    type Error = Error;

    fn try_from(value: Resource) -> Result<Self, Self::Error> {
        let language = value.metadata.language.clone();

        let pairs = value
            .entries
            .iter()
            .map(|entry| Pair::try_from(entry.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Format { language, pairs })
    }
}

/// A single key-value pair in a `.strings` file, possibly with an associated comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    /// The key for this localization entry.
    pub key: String,
    /// The value for this localization entry.
    pub value: String,
    /// Optional comment associated with the key-value pair.
    ///
    /// Only comments that immediately precede a key-value pair are attached to it.
    /// Trailing comments on the same line as a key-value pair (e.g., `"key" = "value"; // comment`)
    /// are ignored and not attached.
    ///
    /// To keep it simple, we only support single-line comments in the form of `// comment` or `/* comment */`.
    /// The comment marker is included in the comment field.
    pub comment: Option<String>,
}

impl Pair {
    fn to_entry(&self) -> Entry {
        Entry {
            id: self.key.clone(),
            value: Translation::Singular(self.value.clone()),
            comment: self.comment.clone(),
            status: if self.value.is_empty() {
                EntryStatus::New
            } else {
                EntryStatus::Translated
            },
            custom: HashMap::new(),
        }
    }
}

impl TryFrom<Entry> for Pair {
    type Error = Error;

    fn try_from(entry: Entry) -> Result<Self, Self::Error> {
        // Strings format only supports singular translations
        // with plain text values.
        match Translation::plain_translation(entry.value) {
            Translation::Singular(value) => Ok(Pair {
                key: entry.id,
                value,
                comment: entry.comment,
            }),
            Translation::Plural(_) => Err(Error::DataMismatch(
                "Plural translations are not supported in .strings format".to_string(),
            )),
        }
    }
}

impl From<Pair> for Entry {
    fn from(pair: Pair) -> Self {
        let is_pair_value_empty = pair.value.is_empty();
        Entry {
            id: pair.key,
            value: Translation::Singular(pair.value),
            comment: pair.comment,
            status: if is_pair_value_empty {
                EntryStatus::New
            } else {
                EntryStatus::Translated
            },
            custom: HashMap::new(),
        }
    }
}

impl Pair {
    // Returns a comment without the comment marker.
    pub fn formatted_comment(&self) -> String {
        if let Some(comment) = &self.comment {
            if comment.starts_with("/*") && comment.ends_with("*/") {
                comment[2..comment.len() - 2].trim().to_string()
            } else if let Some(comment) = comment.strip_prefix("//") {
                comment.trim().to_string()
            } else {
                comment.trim().to_string()
            }
        } else {
            String::new()
        }
    }
}

impl std::fmt::Display for Pair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = format!("\"{}\" = \"{}\";", self.key, self.value);
        if let Some(comment) = &self.comment {
            result.insert_str(0, &format!("{}\n", comment));
        }
        write!(f, "{}", result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Parser;

    #[test]
    fn test_parse_basic_strings_with_comment() {
        let content = r#"
        /* Greeting for the user */
        "hello" = "Hello, world!";
        "#;
        let parsed = Format::from_str(content).unwrap();
        assert_eq!(parsed.pairs.len(), 1);
        let pair = &parsed.pairs[0];
        assert_eq!(pair.key, "hello");
        assert_eq!(pair.value, "Hello, world!");
        assert!(
            pair.comment
                .as_ref()
                .unwrap()
                .contains("Greeting for the user")
        );
    }

    #[test]
    fn test_round_trip_serialization() {
        let content = r#"
        /* Farewell */
        "bye" = "Goodbye!";
        "#;
        let parsed = Format::from_str(content).unwrap();
        let mut output = Vec::new();
        parsed.to_writer(&mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        // Parse again and compare key-value pairs
        let reparsed = Format::from_str(&output_str).unwrap();
        assert_eq!(parsed.pairs.len(), reparsed.pairs.len());
        for (orig, new) in parsed.pairs.iter().zip(reparsed.pairs.iter()) {
            assert_eq!(orig.key, new.key);
            assert_eq!(orig.value, new.value);
        }
    }

    #[test]
    fn test_multiline_value_with_embedded_newlines_and_whitespace() {
        let content = r#"
        /* Multiline value */
        "multiline" = "This is line 1.
            This is line 2.
            This is line 3.";
        "#;
        let parsed = Format::from_str(content).unwrap();
        assert_eq!(parsed.pairs.len(), 1);
        let pair = &parsed.pairs[0];
        assert_eq!(pair.key, "multiline");
        // Should be joined with \n and trimmed of leading spaces on each line
        assert_eq!(
            pair.value,
            "This is line 1.\\nThis is line 2.\\nThis is line 3."
        );
    }

    #[test]
    fn test_blank_lines_and_ignored_malformed_lines() {
        let content = r#"

        // Comment

        "good" = "yes";
        bad line without equals
        "another" = "ok";

        "#;
        let parsed = Format::from_str(content).unwrap();
        assert_eq!(parsed.pairs.len(), 2);
        assert_eq!(parsed.pairs[0].key, "good");
        assert_eq!(parsed.pairs[0].value, "yes");
        assert_eq!(parsed.pairs[1].key, "another");
        assert_eq!(parsed.pairs[1].value, "ok");
    }

    #[test]
    fn test_entry_with_empty_value() {
        let content = r#"
        /* Empty value */
        "empty" = "";
        "#;
        let parsed = Format::from_str(content).unwrap();
        assert_eq!(parsed.pairs.len(), 1);
        let pair = &parsed.pairs[0];
        assert_eq!(pair.key, "empty");
        assert_eq!(pair.value, "");
        // Should be marked as New status in Entry
        let entry = pair.to_entry();
        assert_eq!(entry.status, EntryStatus::New);
    }

    #[test]
    fn test_comments_attached_to_correct_key_value_pairs() {
        let content = r#"
        // Comment for A
        "A" = "a";
        // Comment for B
        "B" = "b";
        /* Block comment for C */
        "C" = "c";
        "#;
        let parsed = Format::from_str(content).unwrap();
        assert_eq!(parsed.pairs.len(), 3);
        let a = &parsed.pairs[0];
        let b = &parsed.pairs[1];
        let c = &parsed.pairs[2];
        assert!(a.comment.as_ref().unwrap().contains("Comment for A"));
        assert!(b.comment.as_ref().unwrap().contains("Comment for B"));
        assert!(c.comment.as_ref().unwrap().contains("Block comment for C"));
    }
}
