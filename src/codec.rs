/// This module provides the `Codec` struct and associated functionality for reading,
/// writing, caching, and loading localized resource files in various formats.
/// The `Codec` struct manages a collection of `Resource` instances and supports
/// format inference, language detection from file paths, and serialization.
///
/// The module handles different localization file formats such as Apple `.strings`,
/// Android XML strings, and `.xcstrings`, providing methods to read from files by type
/// or extension, write resources back to files, and cache resources to JSON.
///
use std::path::Path;

use crate::{error::Error, formats::*, traits::Parser, types::Resource};

/// Represents a collection of localized resources and provides methods to read,
/// write, cache, and load these resources.
pub struct Codec {
    /// The collection of resources managed by this codec.
    pub resources: Box<Vec<Resource>>,
}

impl Codec {
    /// Creates a new, empty `Codec`.
    ///
    /// # Returns
    ///
    /// A new `Codec` instance with no resources.
    pub fn new() -> Self {
        Codec {
            resources: Box::new(Vec::new()),
        }
    }

    /// Reads a resource file given its path and explicit format type.
    ///
    /// # Parameters
    /// - `path`: Path to the resource file.
    /// - `format_type`: The format type of the resource file.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the file was successfully read and resources loaded,
    /// or an `Error` otherwise.
    pub fn read_file_by_type<P: AsRef<Path>>(
        &mut self,
        path: P,
        format_type: FormatType,
    ) -> Result<(), Error> {
        let language = infer_language_from_path(&path, &format_type)?;

        let domain = path
            .as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let path = path.as_ref();

        let mut new_resources = match &format_type {
            FormatType::Strings(_) => {
                vec![Resource::from(StringsFormat::read_from(path)?)]
            }
            FormatType::AndroidStrings(_) => {
                vec![Resource::from(AndroidStringsFormat::read_from(path)?)]
            }
            FormatType::Xcstrings => Vec::<Resource>::try_from(XcstringsFormat::read_from(path)?)?,
        };

        for new_resource in &mut new_resources {
            if let Some(ref lang) = language {
                new_resource.metadata.language = lang.clone();
            }
            new_resource.metadata.domain = domain.clone();
            new_resource
                .metadata
                .custom
                .insert("format".to_string(), format_type.to_string());
        }
        self.resources.append(&mut new_resources);

        Ok(())
    }

    /// Reads a resource file by inferring its format from the file extension.
    /// Optionally infers language from the path if not provided.
    ///
    /// # Parameters
    /// - `path`: Path to the resource file.
    /// - `lang`: Optional language code to use.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the file was successfully read,
    /// or an `Error` if the format is unsupported or reading fails.
    pub fn read_file_by_extension<P: AsRef<Path>>(
        &mut self,
        path: P,
        lang: Option<String>,
    ) -> Result<(), Error> {
        let format_type = match path.as_ref().extension().and_then(|s| s.to_str()) {
            Some("xml") => FormatType::AndroidStrings(lang),
            Some("strings") => FormatType::Strings(lang),
            Some("xcstrings") => FormatType::Xcstrings,
            extension => {
                return Err(Error::UnsupportedFormat(format!(
                    "Unsupported file extension: {:?}.",
                    extension
                )));
            }
        };

        self.read_file_by_type(path, format_type)?;

        Ok(())
    }

    /// Writes all managed resources back to their respective files,
    /// grouped by domain.
    ///
    /// # Returns
    ///
    /// `Ok(())` if all writes succeed, or an `Error` otherwise.
    pub fn write_to_file(&self) -> Result<(), Error> {
        // Group resources by the domain in a HashMap
        let mut grouped_resources: std::collections::HashMap<String, Vec<Resource>> =
            std::collections::HashMap::new();
        for resource in &*self.resources {
            let domain = resource.metadata.domain.clone();
            grouped_resources
                .entry(domain)
                .or_insert_with(Vec::new)
                .push(resource.clone());
        }

        // Iterate the map and write each resource to its respective file
        for (domain, resources) in grouped_resources {
            write_resources_to_file(&resources, &domain)?;
        }

        Ok(())
    }

    /// Caches the current resources to a JSON file.
    ///
    /// # Parameters
    /// - `path`: Destination file path for the cache.
    ///
    /// # Returns
    ///
    /// `Ok(())` if caching succeeds, or an `Error` if file I/O or serialization fails.
    pub fn cache_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let mut writer = std::fs::File::create(path).map_err(Error::Io)?;
        serde_json::to_writer(&mut writer, &*self.resources).map_err(Error::Parse)?;
        Ok(())
    }

    /// Loads resources from a JSON cache file.
    ///
    /// # Parameters
    /// - `path`: Path to the JSON file containing cached resources.
    ///
    /// # Returns
    ///
    /// `Ok(Codec)` with loaded resources, or an `Error` if loading or deserialization fails.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let mut reader = std::fs::File::open(path).map_err(Error::Io)?;
        let resources: Box<Vec<Resource>> =
            serde_json::from_reader(&mut reader).map_err(Error::Parse)?;
        Ok(Codec { resources })
    }
}

/// Attempts to infer the language from the file path based on format conventions.
/// For Apple: looks for "{lang}.lproj"; for Android: "values-{lang}".
///
/// # Parameters
/// - `path`: The file path to analyze.
/// - `format_type`: The format type to consider for language inference.
///
/// # Returns
///
/// `Ok(Some(language_code))` if a language could be inferred,
/// `Ok(None)` if no language is applicable for the format,
/// or an `Error` if inference fails.
fn infer_language_from_path<P: AsRef<Path>>(
    path: &P,
    format_type: &FormatType,
) -> Result<Option<String>, Error> {
    match &format_type {
        FormatType::AndroidStrings(lang) | FormatType::Strings(lang) => {
            let processed_lang = if let Some(lang) = lang {
                lang.clone()
            } else {
                path.as_ref()
                    .components()
                    .rev()
                    .find_map(|c| {
                        let component = c.as_os_str().to_str()?;
                        if component.ends_with(".lproj") {
                            Some(component.trim_end_matches(".lproj").to_string())
                        } else if component.starts_with("values-") {
                            Some(component.trim_start_matches("values-").to_string())
                        } else {
                            None
                        }
                    })
                    .ok_or(Error::UnknownFormat(
                        "Failed to infer language from path, please provide a language code manually."
                            .to_string(),
                    ))?
            };

            Ok(Some(processed_lang))
        }
        _ => Ok(None),
    }
}

/// Writes one or more resources to a file based on their format metadata.
/// Supports formats with single or multiple resources per file.
///
/// # Parameters
/// - `resources`: Slice of resources to write.
/// - `file_path`: Destination file path.
///
/// # Returns
///
/// `Ok(())` if writing succeeds, or an `Error` if the format is unsupported or writing fails.
fn write_resources_to_file(resources: &[Resource], file_path: &String) -> Result<(), Error> {
    let path = Path::new(&file_path);

    if let Some(first) = resources.first() {
        match first.metadata.custom.get("format").map(String::as_str) {
            Some("AndroidStrings") => AndroidStringsFormat::from(first.clone()).write_to(path)?,
            Some("Strings") => StringsFormat::try_from(first.clone())?.write_to(path)?,
            Some("Xcstrings") => XcstringsFormat::try_from(resources.to_vec())?.write_to(path)?,
            _ => Err(Error::UnsupportedFormat(format!(
                "Unsupported format: {:?}",
                first.metadata.custom.get("format")
            )))?,
        }
    }

    Ok(())
}
