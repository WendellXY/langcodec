/// This module provides the `Codec` struct and associated functionality for reading,
/// writing, caching, and loading localized resource files in various formats.
/// The `Codec` struct manages a collection of `Resource` instances and supports
/// format inference, language detection from file paths, and serialization.
///
/// The module handles different localization file formats such as Apple `.strings`,
/// Android XML strings, and `.xcstrings`, providing methods to read from files by type
/// or extension, write resources back to files, and cache resources to JSON.
///
use crate::formats::{CSVFormat, TSVFormat};
use crate::{ConflictStrategy, merge_resources};
use crate::{
    error::Error,
    formats::*,
    traits::Parser,
    types::{Entry, Resource},
};
use std::path::Path;

/// Represents a collection of localized resources and provides methods to read,
/// write, cache, and load these resources.
#[derive(Debug)]
pub struct Codec {
    /// The collection of resources managed by this codec.
    pub resources: Vec<Resource>,
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
    pub fn builder() -> crate::builder::CodecBuilder {
        crate::builder::CodecBuilder::new()
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

    // ===== HIGH-LEVEL MODIFICATION METHODS =====

    /// Finds an entry by its key across all languages.
    ///
    /// Returns an iterator over all resources and their matching entries.
    ///
    /// # Arguments
    ///
    /// * `key` - The entry key to search for
    ///
    /// # Returns
    ///
    /// An iterator yielding `(&Resource, &Entry)` pairs for all matching entries.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::Codec;
    ///
    /// let mut codec = Codec::new();
    /// // ... load resources ...
    ///
    /// for (resource, entry) in codec.find_entries("welcome_message") {
    ///     println!("{}: {}", resource.metadata.language, entry.value);
    /// }
    /// ```
    pub fn find_entries(&self, key: &str) -> Vec<(&Resource, &Entry)> {
        let mut results = Vec::new();
        for resource in &self.resources {
            for entry in &resource.entries {
                if entry.id == key {
                    results.push((resource, entry));
                }
            }
        }
        results
    }

    /// Finds an entry by its key in a specific language.
    ///
    /// # Arguments
    ///
    /// * `key` - The entry key to search for
    /// * `language` - The language code (e.g., "en", "fr")
    ///
    /// # Returns
    ///
    /// `Some(&Entry)` if found, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::Codec;
    ///
    /// let mut codec = Codec::new();
    /// // ... load resources ...
    ///
    /// if let Some(entry) = codec.find_entry("welcome_message", "en") {
    ///     println!("English welcome: {}", entry.value);
    /// }
    /// ```
    pub fn find_entry(&self, key: &str, language: &str) -> Option<&Entry> {
        self.get_by_language(language)?
            .entries
            .iter()
            .find(|entry| entry.id == key)
    }

    /// Finds a mutable entry by its key in a specific language.
    ///
    /// # Arguments
    ///
    /// * `key` - The entry key to search for
    /// * `language` - The language code (e.g., "en", "fr")
    ///
    /// # Returns
    ///
    /// `Some(&mut Entry)` if found, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::Codec;
    /// use langcodec::types::Translation;
    ///
    /// let mut codec = Codec::new();
    /// // ... load resources ...
    ///
    /// if let Some(entry) = codec.find_entry_mut("welcome_message", "en") {
    ///     entry.value = Translation::Singular("Hello, World!".to_string());
    ///     entry.status = langcodec::types::EntryStatus::Translated;
    /// }
    /// ```
    pub fn find_entry_mut(&mut self, key: &str, language: &str) -> Option<&mut Entry> {
        self.get_mut_by_language(language)?
            .entries
            .iter_mut()
            .find(|entry| entry.id == key)
    }

    /// Updates a translation for a specific key and language.
    ///
    /// # Arguments
    ///
    /// * `key` - The entry key to update
    /// * `language` - The language code (e.g., "en", "fr")
    /// * `translation` - The new translation value
    /// * `status` - Optional new status (defaults to `Translated`)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the entry was found and updated, `Err` if not found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::{Codec, types::{Translation, EntryStatus}};
    ///
    /// let mut codec = Codec::new();
    /// // Add an entry first
    /// codec.add_entry("welcome", "en", Translation::Singular("Hello".to_string()), None, None)?;
    ///
    /// codec.update_translation(
    ///     "welcome",
    ///     "en",
    ///     Translation::Singular("Hello, World!".to_string()),
    ///     Some(EntryStatus::Translated)
    /// )?;
    /// # Ok::<(), langcodec::Error>(())
    /// ```
    pub fn update_translation(
        &mut self,
        key: &str,
        language: &str,
        translation: crate::types::Translation,
        status: Option<crate::types::EntryStatus>,
    ) -> Result<(), Error> {
        if let Some(entry) = self.find_entry_mut(key, language) {
            entry.value = translation;
            if let Some(new_status) = status {
                entry.status = new_status;
            }
            Ok(())
        } else {
            Err(Error::InvalidResource(format!(
                "Entry '{}' not found in language '{}'",
                key, language
            )))
        }
    }

    /// Adds a new entry to a specific language.
    ///
    /// If the language doesn't exist, it will be created automatically.
    ///
    /// # Arguments
    ///
    /// * `key` - The entry key
    /// * `language` - The language code (e.g., "en", "fr")
    /// * `translation` - The translation value
    /// * `comment` - Optional comment for translators
    /// * `status` - Optional status (defaults to `New`)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the entry was added successfully.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::{Codec, types::{Translation, EntryStatus}};
    ///
    /// let mut codec = Codec::new();
    ///
    /// codec.add_entry(
    ///     "new_message",
    ///     "en",
    ///     Translation::Singular("This is a new message".to_string()),
    ///     Some("This is a new message for users".to_string()),
    ///     Some(EntryStatus::New)
    /// )?;
    /// # Ok::<(), langcodec::Error>(())
    /// ```
    pub fn add_entry(
        &mut self,
        key: &str,
        language: &str,
        translation: crate::types::Translation,
        comment: Option<String>,
        status: Option<crate::types::EntryStatus>,
    ) -> Result<(), Error> {
        // Find or create the resource for this language
        let resource = if let Some(resource) = self.get_mut_by_language(language) {
            resource
        } else {
            // Create a new resource for this language
            let new_resource = crate::types::Resource {
                metadata: crate::types::Metadata {
                    language: language.to_string(),
                    domain: "".to_string(),
                    custom: std::collections::HashMap::new(),
                },
                entries: Vec::new(),
            };
            self.add_resource(new_resource);
            self.get_mut_by_language(language).unwrap()
        };

        let entry = crate::types::Entry {
            id: key.to_string(),
            value: translation,
            comment,
            status: status.unwrap_or(crate::types::EntryStatus::New),
            custom: std::collections::HashMap::new(),
        };
        resource.add_entry(entry);
        Ok(())
    }

    /// Removes an entry from a specific language.
    ///
    /// # Arguments
    ///
    /// * `key` - The entry key to remove
    /// * `language` - The language code (e.g., "en", "fr")
    ///
    /// # Returns
    ///
    /// `Ok(())` if the entry was found and removed, `Err` if not found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::{Codec, types::{Translation, EntryStatus}};
    ///
    /// let mut codec = Codec::new();
    /// // Add a resource first
    /// codec.add_entry("test_key", "en", Translation::Singular("Test".to_string()), None, None)?;
    ///
    /// // Now remove it
    /// codec.remove_entry("test_key", "en")?;
    /// # Ok::<(), langcodec::Error>(())
    /// ```
    pub fn remove_entry(&mut self, key: &str, language: &str) -> Result<(), Error> {
        if let Some(resource) = self.get_mut_by_language(language) {
            let initial_len = resource.entries.len();
            resource.entries.retain(|entry| entry.id != key);

            if resource.entries.len() == initial_len {
                Err(Error::InvalidResource(format!(
                    "Entry '{}' not found in language '{}'",
                    key, language
                )))
            } else {
                Ok(())
            }
        } else {
            Err(Error::InvalidResource(format!(
                "Language '{}' not found",
                language
            )))
        }
    }

    /// Copies an entry from one language to another.
    ///
    /// This is useful for creating new translations based on existing ones.
    ///
    /// # Arguments
    ///
    /// * `key` - The entry key to copy
    /// * `from_language` - The source language
    /// * `to_language` - The target language
    /// * `update_status` - Whether to update the status to `New` in the target language
    ///
    /// # Returns
    ///
    /// `Ok(())` if the entry was copied successfully, `Err` if not found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::{Codec, types::{Translation, EntryStatus}};
    ///
    /// let mut codec = Codec::new();
    /// // Add source entry first
    /// codec.add_entry("welcome", "en", Translation::Singular("Hello".to_string()), None, None)?;
    ///
    /// // Copy English entry to French as a starting point
    /// codec.copy_entry("welcome", "en", "fr", true)?;
    /// # Ok::<(), langcodec::Error>(())
    /// ```
    pub fn copy_entry(
        &mut self,
        key: &str,
        from_language: &str,
        to_language: &str,
        update_status: bool,
    ) -> Result<(), Error> {
        let source_entry = self.find_entry(key, from_language).ok_or_else(|| {
            Error::InvalidResource(format!(
                "Entry '{}' not found in source language '{}'",
                key, from_language
            ))
        })?;

        let mut new_entry = source_entry.clone();
        if update_status {
            new_entry.status = crate::types::EntryStatus::New;
        }

        // Find or create the target resource
        let target_resource = if let Some(resource) = self.get_mut_by_language(to_language) {
            resource
        } else {
            // Create a new resource for the target language
            let new_resource = crate::types::Resource {
                metadata: crate::types::Metadata {
                    language: to_language.to_string(),
                    domain: "".to_string(),
                    custom: std::collections::HashMap::new(),
                },
                entries: Vec::new(),
            };
            self.add_resource(new_resource);
            self.get_mut_by_language(to_language).unwrap()
        };

        // Remove existing entry if it exists
        target_resource.entries.retain(|entry| entry.id != key);
        target_resource.add_entry(new_entry);
        Ok(())
    }

    /// Gets all languages available in the codec.
    ///
    /// # Returns
    ///
    /// An iterator over all language codes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::Codec;
    ///
    /// let codec = Codec::new();
    /// // ... load resources ...
    ///
    /// for language in codec.languages() {
    ///     println!("Available language: {}", language);
    /// }
    /// ```
    pub fn languages(&self) -> impl Iterator<Item = &str> {
        self.resources.iter().map(|r| r.metadata.language.as_str())
    }

    /// Gets all unique entry keys across all languages.
    ///
    /// # Returns
    ///
    /// An iterator over all unique entry keys.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::Codec;
    ///
    /// let codec = Codec::new();
    /// // ... load resources ...
    ///
    /// for key in codec.all_keys() {
    ///     println!("Available key: {}", key);
    /// }
    /// ```
    pub fn all_keys(&self) -> impl Iterator<Item = &str> {
        use std::collections::HashSet;

        let mut keys = HashSet::new();
        for resource in &self.resources {
            for entry in &resource.entries {
                keys.insert(entry.id.as_str());
            }
        }
        keys.into_iter()
    }

    /// Checks if an entry exists in a specific language.
    ///
    /// # Arguments
    ///
    /// * `key` - The entry key to check
    /// * `language` - The language code (e.g., "en", "fr")
    ///
    /// # Returns
    ///
    /// `true` if the entry exists, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::Codec;
    ///
    /// let codec = Codec::new();
    /// // ... load resources ...
    ///
    /// if codec.has_entry("welcome_message", "en") {
    ///     println!("English welcome message exists");
    /// }
    /// ```
    pub fn has_entry(&self, key: &str, language: &str) -> bool {
        self.find_entry(key, language).is_some()
    }

    /// Gets the count of entries in a specific language.
    ///
    /// # Arguments
    ///
    /// * `language` - The language code (e.g., "en", "fr")
    ///
    /// # Returns
    ///
    /// The number of entries in the specified language, or 0 if the language doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::Codec;
    ///
    /// let codec = Codec::new();
    /// // ... load resources ...
    ///
    /// let count = codec.entry_count("en");
    /// println!("English has {} entries", count);
    /// ```
    pub fn entry_count(&self, language: &str) -> usize {
        self.get_by_language(language)
            .map(|r| r.entries.len())
            .unwrap_or(0)
    }

    /// Validates the codec for common issues.
    ///
    /// # Returns
    ///
    /// `Ok(())` if validation passes, `Err(Error)` with details if validation fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::Codec;
    ///
    /// let mut codec = Codec::new();
    /// // ... add resources ...
    ///
    /// if let Err(validation_error) = codec.validate() {
    ///     eprintln!("Validation failed: {}", validation_error);
    /// }
    /// ```
    pub fn validate(&self) -> Result<(), Error> {
        // Check for empty resources
        if self.resources.is_empty() {
            return Err(Error::InvalidResource("No resources found".to_string()));
        }

        // Check for duplicate languages
        let mut languages = std::collections::HashSet::new();
        for resource in &self.resources {
            if !languages.insert(&resource.metadata.language) {
                return Err(Error::InvalidResource(format!(
                    "Duplicate language found: {}",
                    resource.metadata.language
                )));
            }
        }

        // Check for empty resources
        for resource in &self.resources {
            if resource.entries.is_empty() {
                return Err(Error::InvalidResource(format!(
                    "Resource for language '{}' has no entries",
                    resource.metadata.language
                )));
            }
        }

        Ok(())
    }

    /// Cleans up resources by removing empty resources and entries.
    pub fn clean_up_resources(&mut self) {
        self.resources
            .retain(|resource| !resource.entries.is_empty());
    }

    /// Merge resources with the same language by the given strategy.
    ///
    /// This method groups resources by language and merges multiple resources
    /// that share the same language into a single resource. Resources with
    /// unique languages are left unchanged.
    ///
    /// # Arguments
    ///
    /// * `strategy` - The conflict resolution strategy to use when merging
    ///   entries with the same ID across multiple resources
    ///
    /// # Returns
    ///
    /// The number of merge operations performed. A merge operation occurs
    /// when there are 2 or more resources for the same language.
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::{Codec, types::ConflictStrategy};
    ///
    /// let mut codec = Codec::new();
    /// // ... add resources with same language ...
    ///
    /// let merges_performed = codec.merge_resources(&ConflictStrategy::Last);
    /// println!("Merged {} language groups", merges_performed);
    /// ```
    ///
    /// # Behavior
    ///
    /// - Resources are grouped by language
    /// - Only languages with multiple resources are merged
    /// - The merged resource replaces all original resources for that language
    /// - Resources with unique languages remain unchanged
    /// - Entries are merged according to the specified conflict strategy
    pub fn merge_resources(&mut self, strategy: &ConflictStrategy) -> usize {
        // Group resources by language
        let mut grouped_resources: std::collections::HashMap<String, Vec<Resource>> =
            std::collections::HashMap::new();
        for resource in &self.resources {
            grouped_resources
                .entry(resource.metadata.language.clone())
                .or_default()
                .push(resource.clone());
        }

        let mut merge_count = 0;

        // Merge resources by language
        for (_language, resources) in grouped_resources {
            if resources.len() > 1 {
                match merge_resources(&resources, strategy) {
                    Ok(merged) => {
                        // Replace the original resources with the merged resource and remove the original resources
                        self.resources.retain(|r| r.metadata.language != _language);
                        self.resources.push(merged);
                        merge_count += 1;
                    }
                    Err(e) => {
                        // Based on the current implementation, the merge_resources should never return an error
                        // because we are merging resources with the same language
                        // so we should panic here
                        panic!("Unexpected error merging resources: {}", e);
                    }
                }
            }
        }

        merge_count
    }

    /// Writes a resource to a file with automatic format detection.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to write
    /// * `output_path` - The output file path
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(Error)` on failure.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langcodec::{Codec, types::{Resource, Metadata, Entry, Translation, EntryStatus}};
    ///
    /// let resource = Resource {
    ///     metadata: Metadata {
    ///         language: "en".to_string(),
    ///         domain: "domain".to_string(),
    ///         custom: std::collections::HashMap::new(),
    ///     },
    ///     entries: vec![],
    /// };
    /// Codec::write_resource_to_file(&resource, "output.strings")?;
    /// # Ok::<(), langcodec::Error>(())
    /// ```
    pub fn write_resource_to_file(resource: &Resource, output_path: &str) -> Result<(), Error> {
        use crate::formats::{
            AndroidStringsFormat, CSVFormat, StringsFormat, TSVFormat, XcstringsFormat,
        };
        use std::path::Path;

        // Infer format from output path
        let format_type =
            crate::converter::infer_format_from_extension(output_path).ok_or_else(|| {
                Error::InvalidResource(format!(
                    "Cannot infer format from output path: {}",
                    output_path
                ))
            })?;

        match format_type {
            crate::formats::FormatType::AndroidStrings(_) => {
                AndroidStringsFormat::from(resource.clone())
                    .write_to(Path::new(output_path))
                    .map_err(|e| {
                        Error::conversion_error(
                            format!("Error writing AndroidStrings output: {}", e),
                            None,
                        )
                    })
            }
            crate::formats::FormatType::Strings(_) => StringsFormat::try_from(resource.clone())
                .and_then(|f| f.write_to(Path::new(output_path)))
                .map_err(|e| {
                    Error::conversion_error(format!("Error writing Strings output: {}", e), None)
                }),
            crate::formats::FormatType::Xcstrings => {
                XcstringsFormat::try_from(vec![resource.clone()])
                    .and_then(|f| f.write_to(Path::new(output_path)))
                    .map_err(|e| {
                        Error::conversion_error(
                            format!("Error writing Xcstrings output: {}", e),
                            None,
                        )
                    })
            }
            crate::formats::FormatType::CSV => CSVFormat::try_from(vec![resource.clone()])
                .and_then(|f| f.write_to(Path::new(output_path)))
                .map_err(|e| {
                    Error::conversion_error(format!("Error writing CSV output: {}", e), None)
                }),
            crate::formats::FormatType::TSV => TSVFormat::try_from(vec![resource.clone()])
                .and_then(|f| f.write_to(Path::new(output_path)))
                .map_err(|e| {
                    Error::conversion_error(format!("Error writing TSV output: {}", e), None)
                }),
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
        let language = crate::converter::infer_language_from_path(&path, &format_type)?;

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
            FormatType::CSV => {
                // Parse CSV format and convert to resources
                let csv_format = CSVFormat::read_from(path)?;
                Vec::<Resource>::try_from(csv_format)?
            }
            FormatType::TSV => {
                // Parse TSV format and convert to resources
                let tsv_format = TSVFormat::read_from(path)?;
                Vec::<Resource>::try_from(tsv_format)?
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
            Some("csv") => FormatType::CSV,
            Some("tsv") => FormatType::TSV,
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
            crate::converter::write_resources_to_file(&resources, &domain)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Entry, EntryStatus, Metadata, Translation};
    use std::collections::HashMap;

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

    #[test]
    fn test_modification_methods() {
        use crate::types::{EntryStatus, Translation};

        // Create a codec with some test data
        let mut codec = Codec::new();

        // Add resources
        let resource1 = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "test".to_string(),
                custom: std::collections::HashMap::new(),
            },
            entries: vec![Entry {
                id: "welcome".to_string(),
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
                id: "welcome".to_string(),
                value: Translation::Singular("Bonjour".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: std::collections::HashMap::new(),
            }],
        };

        codec.add_resource(resource1);
        codec.add_resource(resource2);

        // Test find_entries
        let entries = codec.find_entries("welcome");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0.metadata.language, "en");
        assert_eq!(entries[1].0.metadata.language, "fr");

        // Test find_entry
        let entry = codec.find_entry("welcome", "en");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().id, "welcome");

        // Test find_entry_mut and update
        if let Some(entry) = codec.find_entry_mut("welcome", "en") {
            entry.value = Translation::Singular("Hello, World!".to_string());
            entry.status = EntryStatus::NeedsReview;
        }

        // Verify the update
        let updated_entry = codec.find_entry("welcome", "en").unwrap();
        assert_eq!(updated_entry.value.to_string(), "Hello, World!");
        assert_eq!(updated_entry.status, EntryStatus::NeedsReview);

        // Test update_translation
        codec
            .update_translation(
                "welcome",
                "fr",
                Translation::Singular("Bonjour, le monde!".to_string()),
                Some(EntryStatus::NeedsReview),
            )
            .unwrap();

        // Test add_entry
        codec
            .add_entry(
                "new_key",
                "en",
                Translation::Singular("New message".to_string()),
                Some("A new message".to_string()),
                Some(EntryStatus::New),
            )
            .unwrap();

        assert!(codec.has_entry("new_key", "en"));
        assert_eq!(codec.entry_count("en"), 2);

        // Test remove_entry
        codec.remove_entry("new_key", "en").unwrap();
        assert!(!codec.has_entry("new_key", "en"));
        assert_eq!(codec.entry_count("en"), 1);

        // Test copy_entry
        codec.copy_entry("welcome", "en", "fr", true).unwrap();
        let copied_entry = codec.find_entry("welcome", "fr").unwrap();
        assert_eq!(copied_entry.status, EntryStatus::New);

        // Test languages
        let languages: Vec<_> = codec.languages().collect();
        assert_eq!(languages.len(), 2);
        assert!(languages.contains(&"en"));
        assert!(languages.contains(&"fr"));

        // Test all_keys
        let keys: Vec<_> = codec.all_keys().collect();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&"welcome"));
    }

    #[test]
    fn test_validation() {
        let mut codec = Codec::new();

        // Test validation with empty language
        let resource_without_language = Resource {
            metadata: Metadata {
                language: "".to_string(),
                domain: "test".to_string(),
                custom: std::collections::HashMap::new(),
            },
            entries: vec![],
        };

        codec.add_resource(resource_without_language);
        assert!(codec.validate().is_err());

        // Test validation with duplicate languages
        let mut codec = Codec::new();
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

        codec.add_resource(resource1);
        codec.add_resource(resource2);
        assert!(codec.validate().is_err());

        // Test validation with missing translations
        let mut codec = Codec::new();
        let resource1 = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "test".to_string(),
                custom: std::collections::HashMap::new(),
            },
            entries: vec![Entry {
                id: "welcome".to_string(),
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
            entries: vec![], // Missing welcome entry
        };

        codec.add_resource(resource1);
        codec.add_resource(resource2);
        assert!(codec.validate().is_err());
    }

    #[test]
    fn test_convert_csv_to_xcstrings() {
        // Test CSV to XCStrings conversion
        let temp_dir = tempfile::tempdir().unwrap();
        let input_file = temp_dir.path().join("test.csv");
        let output_file = temp_dir.path().join("output.xcstrings");

        let csv_content =
            "key,en,fr,de\nhello,Hello,Bonjour,Hallo\nbye,Goodbye,Au revoir,Auf Wiedersehen\n";
        std::fs::write(&input_file, csv_content).unwrap();

        // First, let's see what the CSV parsing produces
        let csv_format = CSVFormat::read_from(&input_file).unwrap();
        let resources = Vec::<Resource>::try_from(csv_format).unwrap();
        println!("CSV parsed to {} resources:", resources.len());
        for (i, resource) in resources.iter().enumerate() {
            println!(
                "  Resource {}: language={}, entries={}",
                i,
                resource.metadata.language,
                resource.entries.len()
            );
            for entry in &resource.entries {
                println!("    Entry: id={}, value={:?}", entry.id, entry.value);
            }
        }

        let result = crate::converter::convert(
            &input_file,
            FormatType::CSV,
            &output_file,
            FormatType::Xcstrings,
        );

        match result {
            Ok(()) => println!("✅ CSV to XCStrings conversion succeeded"),
            Err(e) => println!("❌ CSV to XCStrings conversion failed: {}", e),
        }

        // Check the output file content
        if output_file.exists() {
            let content = std::fs::read_to_string(&output_file).unwrap();
            println!("Output file content: {}", content);
        }

        // Clean up
        let _ = std::fs::remove_file(input_file);
        let _ = std::fs::remove_file(output_file);
    }

    #[test]
    fn test_merge_resources_method() {
        use crate::types::{ConflictStrategy, Entry, EntryStatus, Metadata, Translation};

        let mut codec = Codec::new();

        // Create multiple resources for the same language (English)
        let resource1 = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "domain1".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "hello".to_string(),
                value: Translation::Singular("Hello".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        let resource2 = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "domain2".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "goodbye".to_string(),
                value: Translation::Singular("Goodbye".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        let resource3 = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "domain3".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "hello".to_string(), // Conflict with resource1
                value: Translation::Singular("Hi".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        // Add resources to codec
        codec.add_resource(resource1);
        codec.add_resource(resource2);
        codec.add_resource(resource3);

        // Test merging with Last strategy
        let merges_performed = codec.merge_resources(&ConflictStrategy::Last);
        assert_eq!(merges_performed, 1); // Should merge 1 language group
        assert_eq!(codec.resources.len(), 1); // Should have 1 merged resource

        let merged_resource = &codec.resources[0];
        assert_eq!(merged_resource.metadata.language, "en");
        assert_eq!(merged_resource.entries.len(), 2); // hello + goodbye

        // Check that the last entry for "hello" was kept (from resource3)
        let hello_entry = merged_resource
            .entries
            .iter()
            .find(|e| e.id == "hello")
            .unwrap();
        assert_eq!(hello_entry.value.plain_translation_string(), "Hi");
    }

    #[test]
    fn test_merge_resources_method_multiple_languages() {
        use crate::types::{ConflictStrategy, Entry, EntryStatus, Metadata, Translation};

        let mut codec = Codec::new();

        // Create resources for English (multiple)
        let en_resource1 = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "domain1".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "hello".to_string(),
                value: Translation::Singular("Hello".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        let en_resource2 = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "domain2".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "goodbye".to_string(),
                value: Translation::Singular("Goodbye".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        // Create resource for French (single)
        let fr_resource = Resource {
            metadata: Metadata {
                language: "fr".to_string(),
                domain: "domain1".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "bonjour".to_string(),
                value: Translation::Singular("Bonjour".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        // Add resources to codec
        codec.add_resource(en_resource1);
        codec.add_resource(en_resource2);
        codec.add_resource(fr_resource);

        // Test merging
        let merges_performed = codec.merge_resources(&ConflictStrategy::First);
        assert_eq!(merges_performed, 1); // Should merge 1 language group (English)
        assert_eq!(codec.resources.len(), 2); // Should have 2 resources (merged English + French)

        // Check English resource was merged
        let en_resource = codec.get_by_language("en").unwrap();
        assert_eq!(en_resource.entries.len(), 2);

        // Check French resource was unchanged
        let fr_resource = codec.get_by_language("fr").unwrap();
        assert_eq!(fr_resource.entries.len(), 1);
        assert_eq!(fr_resource.entries[0].id, "bonjour");
    }

    #[test]
    fn test_merge_resources_method_no_merges() {
        use crate::types::{ConflictStrategy, Entry, EntryStatus, Metadata, Translation};

        let mut codec = Codec::new();

        // Create resources for different languages (no conflicts)
        let en_resource = Resource {
            metadata: Metadata {
                language: "en".to_string(),
                domain: "domain1".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "hello".to_string(),
                value: Translation::Singular("Hello".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        let fr_resource = Resource {
            metadata: Metadata {
                language: "fr".to_string(),
                domain: "domain1".to_string(),
                custom: HashMap::new(),
            },
            entries: vec![Entry {
                id: "bonjour".to_string(),
                value: Translation::Singular("Bonjour".to_string()),
                comment: None,
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        // Add resources to codec
        codec.add_resource(en_resource);
        codec.add_resource(fr_resource);

        // Test merging - should perform no merges
        let merges_performed = codec.merge_resources(&ConflictStrategy::Last);
        assert_eq!(merges_performed, 0); // Should merge 0 language groups
        assert_eq!(codec.resources.len(), 2); // Should still have 2 resources

        // Check resources are unchanged
        assert!(codec.get_by_language("en").is_some());
        assert!(codec.get_by_language("fr").is_some());
    }

    #[test]
    fn test_merge_resources_method_empty_codec() {
        let mut codec = Codec::new();

        // Test merging empty codec
        let merges_performed = codec.merge_resources(&ConflictStrategy::Last);
        assert_eq!(merges_performed, 0);
        assert_eq!(codec.resources.len(), 0);
    }
}
