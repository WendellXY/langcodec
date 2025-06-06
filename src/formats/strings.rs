use std::collections::HashMap;

use indoc::indoc;

use crate::{
    error::Error,
    traits::Parser,
    types::{Entry, EntryStatus, Metadata, Resource, Translation},
};

/// A parser for the Apple's .strings format.
///
/// The .strings format is a simple key-value pair format used for localization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    pub language: String,
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

                Some(Pair {
                    key,
                    value,
                    comment: match last_comment {
                        Some(comment) if comment.starts_with("/*") || comment.starts_with("//") => {
                            Some(comment.trim().to_string())
                        }
                        _ => None,
                    },
                })
            })
            .collect();

        // Extract language from header if available
        let language = &header
            .get("Language")
            .cloned()
            .unwrap_or_else(|| String::new());

        Ok(Format {
            language: language.to_string(), // .strings format does not have a language field
            pairs: pairs,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub key: String,
    pub value: String,
    /// Optional comment associated with the key-value pair.
    ///
    /// To keep it simple, we only support single-line comments in the form of `// comment` or `/* comment */`.
    /// And the comment marker is included in the comment field.
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
                value: value,
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
            } else if comment.starts_with("//") {
                comment[2..].trim().to_string()
            } else {
                comment.trim().to_string()
            }
        } else {
            String::new()
        }
    }

    fn to_string(&self) -> String {
        let mut result = format!("\"{}\" = \"{}\";", self.key, self.value);
        if let Some(comment) = &self.comment {
            result.insert_str(0, &format!("{}\n", comment));
        }
        result
    }
}
