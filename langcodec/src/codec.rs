/// This module provides the `Codec` struct and associated functionality for reading,
/// writing, caching, and loading localized resource files in various formats.
/// The `Codec` struct manages a collection of `Resource` instances and supports
/// format inference, language detection from file paths, and serialization.
///
/// The module handles different localization file formats such as Apple `.strings`,
/// Android XML strings, and `.xcstrings`, providing methods to read from files by type
/// or extension, write resources back to files, and cache resources to JSON.
///
use crate::formats::CSVRecord;
use crate::{error::Error, formats::*, traits::Parser, types::Resource};
use std::path::Path;

/// Represents a collection of localized resources and provides methods to read,
/// write, cache, and load these resources.
#[derive(Debug)]
pub struct Codec {
    /// The collection of resources managed by this codec.
    pub resources: Vec<Resource>,
}

/// Builder for creating a `Codec` instance with a fluent interface.
///
/// This builder allows you to chain method calls to add resources from files
/// and then build the final `Codec` instance.
///
/// # Example
///
/// ```rust,no_run
/// use langcodec::{Codec, formats::FormatType};
///
/// let codec = Codec::builder()
///     .add_file("en.strings")?
///     .add_file("fr.strings")?
///     .add_file_with_format("de.xml", FormatType::AndroidStrings(Some("de".to_string())))?
///     .build();
/// # Ok::<(), langcodec::Error>(())
/// ```
pub struct CodecBuilder {
    resources: Vec<Resource>,
}

impl CodecBuilder {
    /// Creates a new `CodecBuilder` with no resources.
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
        }
    }

    /// Adds a resource file by inferring its format from the file extension.
    ///
    /// The language will be automatically inferred from the file path if possible.
    /// For example, `en.lproj/Localizable.strings` will be detected as English.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the resource file
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining, or an `Error` if the file cannot be read.
    pub fn add_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Error> {
        let path = path.as_ref();
        let format_type = infer_format_from_path(path).ok_or_else(|| {
            Error::UnknownFormat(format!(
                "Cannot infer format from file extension: {:?}",
                path.extension()
            ))
        })?;

        let language = infer_language_from_path(&path, &format_type)?;
        let domain = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();

        let mut new_resources = match &format_type {
            FormatType::Strings(_) => {
                vec![Resource::from(StringsFormat::read_from(path)?)]
            }
            FormatType::AndroidStrings(_) => {
                vec![Resource::from(AndroidStringsFormat::read_from(path)?)]
            }
            FormatType::Xcstrings => Vec::<Resource>::try_from(XcstringsFormat::read_from(path)?)?,
            FormatType::CSV(_) => {
                vec![Resource::from(Vec::<CSVRecord>::read_from(path)?)]
            }
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

        self.resources.extend(new_resources);
        Ok(self)
    }

    /// Adds a resource file with an explicit format type.
    ///
    /// This method allows you to specify the format type explicitly, which is useful
    /// when the file extension doesn't match the expected format or when you want
    /// to override the automatic format detection.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the resource file
    /// * `format_type` - The format type of the resource file
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining, or an `Error` if the file cannot be read.
    pub fn add_file_with_format<P: AsRef<Path>>(
        mut self,
        path: P,
        format_type: FormatType,
    ) -> Result<Self, Error> {
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
            FormatType::CSV(_) => {
                vec![Resource::from(Vec::<CSVRecord>::read_from(path)?)]
            }
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

        self.resources.extend(new_resources);
        Ok(self)
    }

    /// Adds a resource directly to the builder.
    ///
    /// This method allows you to add a `Resource` instance directly, which is useful
    /// when you have resources that were created programmatically or loaded from
    /// other sources.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to add
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining.
    pub fn add_resource(mut self, resource: Resource) -> Self {
        self.resources.push(resource);
        self
    }

    /// Adds multiple resources directly to the builder.
    ///
    /// This method allows you to add multiple `Resource` instances at once.
    ///
    /// # Arguments
    ///
    /// * `resources` - Iterator of resources to add
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining.
    pub fn add_resources<I>(mut self, resources: I) -> Self
    where
        I: IntoIterator<Item = Resource>,
    {
        self.resources.extend(resources);
        self
    }

    /// Loads resources from a JSON cache file.
    ///
    /// This method loads resources that were previously cached using `Codec::cache_to_file`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON cache file
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining, or an `Error` if the file cannot be read.
    pub fn load_from_cache<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Error> {
        let mut reader = std::fs::File::open(path).map_err(Error::Io)?;
        let cached_resources: Vec<Resource> =
            serde_json::from_reader(&mut reader).map_err(Error::Parse)?;
        self.resources.extend(cached_resources);
        Ok(self)
    }

    /// Builds the final `Codec` instance.
    ///
    /// This method consumes the builder and returns the constructed `Codec`.
    ///
    /// # Returns
    ///
    /// Returns the constructed `Codec` instance.
    pub fn build(self) -> Codec {
        Codec {
            resources: self.resources,
        }
    }

    /// Builds the final `Codec` instance and validates it.
    ///
    /// This method is similar to `build()` but performs additional validation
    /// on the resources before returning the `Codec`.
    ///
    /// # Returns
    ///
    /// Returns the constructed `Codec` instance, or an `Error` if validation fails.
    pub fn build_and_validate(self) -> Result<Codec, Error> {
        let codec = self.build();

        // Validate that all resources have a language
        for (i, resource) in codec.resources.iter().enumerate() {
            if resource.metadata.language.is_empty() {
                return Err(Error::Validation(format!(
                    "Resource at index {} has no language specified",
                    i
                )));
            }
        }

        // Check for duplicate languages
        let mut languages = std::collections::HashSet::new();
        for resource in &codec.resources {
            if !languages.insert(&resource.metadata.language) {
                return Err(Error::Validation(format!(
                    "Duplicate language found: {}",
                    resource.metadata.language
                )));
            }
        }

        Ok(codec)
    }
}

impl Default for CodecBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Codec {
    fn default() -> Self {
        Codec::new()
    }
}

impl Codec {
    /// Creates a new, empty `Codec`.
    ///
    /// # Returns
    ///
    /// A new `Codec` instance with no resources.
    pub fn new() -> Self {
        Codec {
            resources: Vec::new(),
        }
    }

    /// Creates a new `CodecBuilder` for fluent construction.
    ///
    /// This method returns a builder that allows you to chain method calls
    /// to add resources from files and then build the final `Codec` instance.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langcodec::Codec;
    ///
    /// let codec = Codec::builder()
    ///     .add_file("en.strings")?
    ///     .add_file("fr.strings")?
    ///     .build();
    /// # Ok::<(), langcodec::Error>(())
    /// ```
    ///
    /// # Returns
    ///
    /// Returns a new `CodecBuilder` instance.
    pub fn builder() -> CodecBuilder {
        CodecBuilder::new()
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
            FormatType::CSV(_) => {
                vec![Resource::from(Vec::<CSVRecord>::read_from(path)?)]
            }
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
            Some("csv") => FormatType::CSV(lang),
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
                .or_default()
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
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }
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
        let resources: Vec<Resource> =
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
        FormatType::AndroidStrings(lang) | FormatType::Strings(lang) | FormatType::CSV(lang) => {
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
            Some("CSV") => Vec::<CSVRecord>::try_from(first.clone())?.write_to(path)?,
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

    // Propagate language code from input to output format if not specified
    let output_format = if let Some(lang) = input_format.language() {
        output_format.with_language(Some(lang.clone()))
    } else {
        output_format
    };

    if !input_format.matches_language_of(&output_format) {
        return Err(Error::InvalidResource(
            "Input and output formats must match in language.".to_string(),
        ));
    }

    // Read input as resources
    let resources = match input_format {
        FormatType::AndroidStrings(_) => vec![AndroidStringsFormat::read_from(&input)?.into()],
        FormatType::Strings(_) => vec![StringsFormat::read_from(&input)?.into()],
        FormatType::Xcstrings => {
            Vec::<crate::types::Resource>::try_from(XcstringsFormat::read_from(&input)?)?
        }
        FormatType::CSV(_) => vec![Vec::<CSVRecord>::read_from(&input)?.into()],
    };

    // Helper to extract resource by language if present, or first one
    let pick_resource = |lang: Option<String>| -> Option<crate::types::Resource> {
        match lang {
            Some(l) => resources.iter().find(|r| r.metadata.language == l).cloned(),
            None => resources.first().cloned(),
        }
    };

    match output_format {
        FormatType::AndroidStrings(lang) => {
            let resource = pick_resource(lang);
            if let Some(res) = resource {
                AndroidStringsFormat::from(res).write_to(&output)
            } else {
                Err(Error::InvalidResource(
                    "No matching resource for output language.".to_string(),
                ))
            }
        }
        FormatType::Strings(lang) => {
            let resource = pick_resource(lang);
            if let Some(res) = resource {
                StringsFormat::try_from(res)?.write_to(&output)
            } else {
                Err(Error::InvalidResource(
                    "No matching resource for output language.".to_string(),
                ))
            }
        }
        FormatType::Xcstrings => XcstringsFormat::try_from(resources)?.write_to(&output),
        FormatType::CSV(lang) => {
            let resource = pick_resource(lang);
            if let Some(res) = resource {
                Vec::<CSVRecord>::try_from(res)?.write_to(&output)
            } else {
                Err(Error::InvalidResource(
                    "No matching resource for output language.".to_string(),
                ))
            }
        }
    }
}

/// Infers a [`FormatType`] from a file path's extension.
///
/// Returns `Some(FormatType)` if the extension matches a known format, otherwise `None`.
///
/// # Example
/// ```rust
/// use langcodec::formats::FormatType;
/// use langcodec::codec::infer_format_from_extension;
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
        Some("csv") => Some(FormatType::CSV(None)),
        _ => None,
    }
}

/// Infers the localization file format and language code from a path.
///
/// - For Apple `.strings`: extracts language from `??.lproj/` (e.g. `en.lproj/Localizable.strings`)
/// - For Android `strings.xml`: extracts language from `values-??/` (e.g. `values-es/strings.xml`)
/// - For `.xcstrings`: returns format without language info (contained in file)
///
/// # Examples
/// ```rust
/// use langcodec::formats::FormatType;
/// use langcodec::codec::infer_format_from_path;
/// assert_eq!(
///    infer_format_from_path("ar.lproj/Localizable.strings"),
///    Some(FormatType::Strings(Some("ar".to_string())))
/// );
/// assert_eq!(
///     infer_format_from_path("en.lproj/Localizable.strings"),
///     Some(FormatType::Strings(Some("en".to_string())))
/// );
/// assert_eq!(
///     infer_format_from_path("Base.lproj/Localizable.strings"),
///     Some(FormatType::Strings(Some("Base".to_string())))
/// );
/// assert_eq!(
///     infer_format_from_path("values-es/strings.xml"),
///     Some(FormatType::AndroidStrings(Some("es".to_string())))
/// );
/// assert_eq!(
///     infer_format_from_path("values/strings.xml"),
///     Some(FormatType::AndroidStrings(None))
/// );
/// assert_eq!(
///     infer_format_from_path("Localizable.xcstrings"),
///     Some(FormatType::Xcstrings)
/// );
/// ```
pub fn infer_format_from_path<P: AsRef<Path>>(path: P) -> Option<FormatType> {
    match infer_format_from_extension(&path) {
        Some(format) => match format {
            FormatType::Xcstrings => Some(format),
            FormatType::AndroidStrings(_) | FormatType::Strings(_) | FormatType::CSV(_) => {
                let lang = infer_language_from_path(&path, &format).ok().flatten();
                Some(format.with_language(lang))
            }
        },
        None => None,
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
    let input_format = infer_format_from_path(&input).ok_or_else(|| {
        Error::UnknownFormat(format!(
            "Cannot infer input format from extension: {:?}",
            input.as_ref().extension()
        ))
    })?;
    let output_format = infer_format_from_path(&output).ok_or_else(|| {
        Error::UnknownFormat(format!(
            "Cannot infer output format from extension: {:?}",
            output.as_ref().extension()
        ))
    })?;
    convert(input, input_format, output, output_format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Entry, EntryStatus, Metadata, Translation};

    #[test]
    fn test_builder_pattern() {
        // Test creating an empty codec
        let codec = Codec::builder().build();
        assert_eq!(codec.resources.len(), 0);

        // Test adding resources directly
        let resource1 = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "test".to_string(),
                custom: std::collections::HashMap::new(),
            },
            entries: vec![Entry {
                id: "hello".to_string(),
                value: Translation::Singular("Hello".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: std::collections::HashMap::new(),
            }],
        };

        let resource2 = Resource {
            metadata: Metadata {
                language: "fr".to_string(),
                domain: "test".to_string(),
                custom: std::collections::HashMap::new(),
            },
            entries: vec![Entry {
                id: "hello".to_string(),
                value: Translation::Singular("Bonjour".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: std::collections::HashMap::new(),
            }],
        };

        let codec = Codec::builder()
            .add_resource(resource1.clone())
            .add_resource(resource2.clone())
            .build();

        assert_eq!(codec.resources.len(), 2);
        assert_eq!(codec.resources[0].metadata.language, "en");
        assert_eq!(codec.resources[1].metadata.language, "fr");
    }

    #[test]
    fn test_builder_validation() {
        // Test validation with empty language
        let resource_without_language = Resource {
            metadata: Metadata {
                language: "".to_string(),
                domain: "test".to_string(),
                custom: std::collections::HashMap::new(),
            },
            entries: vec![],
        };

        let result = Codec::builder()
            .add_resource(resource_without_language)
            .build_and_validate();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Validation(_)));

        // Test validation with duplicate languages
        let resource1 = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "test".to_string(),
                custom: std::collections::HashMap::new(),
            },
            entries: vec![],
        };

        let resource2 = Resource {
            metadata: Metadata {
                language: "en".to_string(), // Duplicate language
                domain: "test".to_string(),
                custom: std::collections::HashMap::new(),
            },
            entries: vec![],
        };

        let result = Codec::builder()
            .add_resource(resource1)
            .add_resource(resource2)
            .build_and_validate();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Validation(_)));
    }

    #[test]
    fn test_builder_add_resources() {
        let resources = vec![
            Resource {
                metadata: Metadata {
                    language: "en".to_string(),
                    domain: "test".to_string(),
                    custom: std::collections::HashMap::new(),
                },
                entries: vec![],
            },
            Resource {
                metadata: Metadata {
                    language: "fr".to_string(),
                    domain: "test".to_string(),
                    custom: std::collections::HashMap::new(),
                },
                entries: vec![],
            },
        ];

        let codec = Codec::builder().add_resources(resources).build();
        assert_eq!(codec.resources.len(), 2);
        assert_eq!(codec.resources[0].metadata.language, "en");
        assert_eq!(codec.resources[1].metadata.language, "fr");
    }
}
