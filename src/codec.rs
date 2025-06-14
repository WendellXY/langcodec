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

    /// Returns an iterator over all resources.
    pub fn iter(&self) -> std::slice::Iter<Resource> {
        self.resources.iter()
    }

    /// Returns a mutable iterator over all resources.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<Resource> {
        self.resources.iter_mut()
    }

    /// Finds a resource by its language code, if present.
    pub fn get_by_language(&self, lang: &str) -> Option<&Resource> {
        self.resources
            .iter()
            .find(|res| res.metadata.language == lang)
    }

    /// Finds a mutable resource by its language code, if present.
    pub fn get_mut_by_language(&mut self, lang: &str) -> Option<&mut Resource> {
        self.resources
            .iter_mut()
            .find(|res| res.metadata.language == lang)
    }

    /// Adds a new resource to the collection.
    pub fn add_resource(&mut self, resource: Resource) {
        self.resources.push(resource);
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

/// Convert a localization file from one format to another.
///
/// # Arguments
///
/// * `input` - The input file path.
/// * `input_format` - The format of the input file.
/// * `output` - The output file path.
/// * `output_format` - The format of the output file.
///
/// # Errors
///
/// Returns an `Error` if reading, parsing, converting, or writing fails.
///
/// # Example
///
/// ```rust,no_run
/// use langcodec::{convert, formats::FormatType};
/// convert(
///     "Localizable.strings",
///     FormatType::Strings(None),
///     "strings.xml",
///     FormatType::AndroidStrings(None),
/// )?;
/// # Ok::<(), langcodec::Error>(())
/// ```
pub fn convert<P: AsRef<Path>>(
    input: P,
    input_format: FormatType,
    output: P,
    output_format: FormatType,
) -> Result<(), Error> {
    use crate::formats::{AndroidStringsFormat, StringsFormat, XcstringsFormat};
    use crate::traits::Parser;

    // Read input file as internal Resource(s)
    let resources = match input_format {
        FormatType::AndroidStrings(_) => {
            let format = AndroidStringsFormat::read_from(&input)?;
            vec![format.into()]
        }
        FormatType::Strings(_) => {
            let format = StringsFormat::read_from(&input)?;
            vec![format.into()]
        }
        FormatType::Xcstrings => {
            let format = XcstringsFormat::read_from(&input)?;
            Vec::<crate::types::Resource>::try_from(format)?
        }
    };

    // Convert to target format and write output
    match output_format {
        FormatType::AndroidStrings(_) => {
            let format = AndroidStringsFormat::from(resources.into_iter().next().unwrap());
            format.write_to(&output)
        }
        FormatType::Strings(_) => {
            let format = StringsFormat::try_from(resources.into_iter().next().unwrap())?;
            format.write_to(&output)
        }
        FormatType::Xcstrings => {
            let format = XcstringsFormat::try_from(resources)?;
            format.write_to(&output)
        }
    }
}

/// Infers a [`FormatType`] from a file path's extension.
///
/// Returns `Some(FormatType)` if the extension matches a known format, otherwise `None`.
///
/// # Example
/// ```
/// use langcodec::formats::FormatType;
/// # use langcodec::codec::infer_format_from_extension;
/// assert_eq!(
///     infer_format_from_extension("foo.strings"),
///     Some(FormatType::Strings(None))
/// );
/// assert_eq!(
///     infer_format_from_extension("foo.xml"),
///     Some(FormatType::AndroidStrings(None))
/// );
/// assert_eq!(
///     infer_format_from_extension("foo.xcstrings"),
///     Some(FormatType::Xcstrings)
/// );
/// assert_eq!(
///     infer_format_from_extension("foo.txt"),
///     None
/// );
/// ```
pub fn infer_format_from_extension<P: AsRef<Path>>(path: P) -> Option<FormatType> {
    match path.as_ref().extension().and_then(|s| s.to_str()) {
        Some("xml") => Some(FormatType::AndroidStrings(None)),
        Some("strings") => Some(FormatType::Strings(None)),
        Some("xcstrings") => Some(FormatType::Xcstrings),
        _ => None,
    }
}

/// Convert a localization file from one format to another, inferring formats from file extensions.
///
/// This function attempts to infer the input and output formats from their file extensions.
/// Returns an error if either format cannot be inferred.
///
/// # Arguments
///
/// * `input` - The input file path.
/// * `output` - The output file path.
///
/// # Errors
///
/// Returns an `Error` if the format cannot be inferred, or if conversion fails.
///
/// # Example
///
/// ```rust,no_run
/// use langcodec::convert_auto;
/// convert_auto("Localizable.strings", "strings.xml")?;
/// # Ok::<(), langcodec::Error>(())
/// ```
pub fn convert_auto<P: AsRef<Path>>(input: P, output: P) -> Result<(), Error> {
    let input_format = infer_format_from_extension(&input).ok_or_else(|| {
        Error::UnknownFormat(format!(
            "Cannot infer input format from extension: {:?}",
            input.as_ref().extension()
        ))
    })?;
    let output_format = infer_format_from_extension(&output).ok_or_else(|| {
        Error::UnknownFormat(format!(
            "Cannot infer output format from extension: {:?}",
            output.as_ref().extension()
        ))
    })?;
    convert(input, input_format, output, output_format)
}
