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

    /// Merges multiple resources into a single resource with conflict resolution.
    ///
    /// This function merges resources that all have the same language.
    /// Only entries with the same ID are treated as conflicts.
    ///
    /// # Arguments
    ///
    /// * `resources` - The resources to merge (must all have the same language)
    /// * `conflict_strategy` - How to handle conflicting entries (same ID)
    ///
    /// # Returns
    ///
    /// A merged resource with all entries from the input resources.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No resources are provided
    /// - Resources have different languages (each Resource represents one language)
    ///
    /// # Example
    ///
    /// ```rust
    /// use langcodec::{Codec, types::{Resource, Metadata, Entry, Translation, EntryStatus}};
    ///
    /// let mut codec = Codec::new();
    /// // ... load resources ...
    ///
    /// // Create some sample resources for merging
    /// let resource1 = Resource {
    ///     metadata: Metadata {
    ///         language: "en".to_string(),
    ///         domain: "domain".to_string(),
    ///         custom: std::collections::HashMap::new(),
    ///     },
    ///     entries: vec![
    ///         Entry {
    ///             id: "hello".to_string(),
    ///             value: Translation::Singular("Hello".to_string()),
    ///             comment: None,
    ///             status: EntryStatus::Translated,
    ///             custom: std::collections::HashMap::new(),
    ///         }
    ///     ],
    /// };
    ///
    /// let merged = Codec::merge_resources(
    ///     &[resource1],
    ///     langcodec::types::ConflictStrategy::Last
    /// )?;
    /// # Ok::<(), langcodec::Error>(())
    /// ```
    pub fn merge_resources(
        resources: &[Resource],
        conflict_strategy: crate::types::ConflictStrategy,
    ) -> Result<Resource, Error> {
        if resources.is_empty() {
            return Err(Error::InvalidResource("No resources to merge".to_string()));
        }

        // Validate that all resources have the same language
        let first_language = &resources[0].metadata.language;
        for (i, resource) in resources.iter().enumerate() {
            if resource.metadata.language != *first_language {
                return Err(Error::InvalidResource(format!(
                    "Cannot merge resources with different languages: resource {} has language '{}', but first resource has language '{}'",
                    i + 1,
                    resource.metadata.language,
                    first_language
                )));
            }
        }

        let mut merged = resources[0].clone();
        let mut all_entries = std::collections::HashMap::new();

        // Collect all entries from all resources
        for resource in resources {
            for entry in &resource.entries {
                // Use the original entry ID for conflict resolution
                // Since all resources have the same language, conflicts are based on ID only
                match conflict_strategy {
                    crate::types::ConflictStrategy::First => {
                        all_entries
                            .entry(&entry.id)
                            .or_insert_with(|| entry.clone());
                    }
                    crate::types::ConflictStrategy::Last => {
                        all_entries.insert(&entry.id, entry.clone());
                    }
                    crate::types::ConflictStrategy::Skip => {
                        if all_entries.contains_key(&entry.id) {
                            // Remove the existing entry and skip this one too
                            all_entries.remove(&entry.id);
                            continue;
                        }
                        all_entries.insert(&entry.id, entry.clone());
                    }
                }
            }
        }

        // Convert back to vector and sort by key for consistent output
        merged.entries = all_entries.into_values().collect();
        merged.entries.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(merged)
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
        use crate::formats::{AndroidStringsFormat, CSVRecord, StringsFormat, XcstringsFormat};
        use std::path::Path;

        // Infer format from output path
        let format_type = infer_format_from_extension(output_path).ok_or_else(|| {
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
            crate::formats::FormatType::CSV(_) => Vec::<CSVRecord>::try_from(resource.clone())
                .and_then(|f| f.write_to(Path::new(output_path)))
                .map_err(|e| {
                    Error::conversion_error(format!("Error writing CSV output: {}", e), None)
                }),
        }
    }

    /// Converts a vector of resources to a specific output format.
    ///
    /// # Arguments
    ///
    /// * `resources` - The resources to convert
    /// * `output_path` - The output file path
    /// * `output_format` - The target format
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(Error)` on failure.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// use langcodec::{Codec, types::{Resource, Metadata, Entry, Translation, EntryStatus}, formats::FormatType};
    ///
    /// let resources = vec![Resource {
    ///     metadata: Metadata {
    ///         language: "en".to_string(),
    ///         domain: "domain".to_string(),
    ///         custom: std::collections::HashMap::new(),
    ///     },
    ///     entries: vec![],
    /// }];
    /// Codec::convert_resources_to_format(
    ///     resources,
    ///     "output.strings",
    ///     FormatType::Strings(None)
    /// )?;
    /// # Ok::<(), langcodec::Error>(())
    /// ```
    pub fn convert_resources_to_format(
        resources: Vec<Resource>,
        output_path: &str,
        output_format: crate::formats::FormatType,
    ) -> Result<(), Error> {
        use crate::formats::{AndroidStringsFormat, CSVRecord, StringsFormat, XcstringsFormat};
        use std::path::Path;

        match output_format {
            crate::formats::FormatType::AndroidStrings(_) => {
                if let Some(resource) = resources.first() {
                    AndroidStringsFormat::from(resource.clone())
                        .write_to(Path::new(output_path))
                        .map_err(|e| {
                            Error::conversion_error(
                                format!("Error writing AndroidStrings output: {}", e),
                                None,
                            )
                        })
                } else {
                    Err(Error::InvalidResource(
                        "No resources to convert".to_string(),
                    ))
                }
            }
            crate::formats::FormatType::Strings(_) => {
                if let Some(resource) = resources.first() {
                    StringsFormat::try_from(resource.clone())
                        .and_then(|f| f.write_to(Path::new(output_path)))
                        .map_err(|e| {
                            Error::conversion_error(
                                format!("Error writing Strings output: {}", e),
                                None,
                            )
                        })
                } else {
                    Err(Error::InvalidResource(
                        "No resources to convert".to_string(),
                    ))
                }
            }
            crate::formats::FormatType::Xcstrings => XcstringsFormat::try_from(resources)
                .and_then(|f| f.write_to(Path::new(output_path)))
                .map_err(|e| {
                    Error::conversion_error(format!("Error writing Xcstrings output: {}", e), None)
                }),
            crate::formats::FormatType::CSV(_) => {
                if let Some(resource) = resources.first() {
                    Vec::<CSVRecord>::try_from(resource.clone())
                        .and_then(|f| f.write_to(Path::new(output_path)))
                        .map_err(|e| {
                            Error::conversion_error(
                                format!("Error writing CSV output: {}", e),
                                None,
                            )
                        })
                } else {
                    Err(Error::InvalidResource(
                        "No resources to convert".to_string(),
                    ))
                }
            }
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
pub fn infer_language_from_path<P: AsRef<Path>>(
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
}
