//! Format conversion utilities for langcodec.
//!
//! This module provides functions for converting between different localization file formats,
//! format inference, and utility functions for working with resources.

use crate::{
    ConflictStrategy,
    error::Error,
    formats::{
        AndroidStringsFormat, CSVFormat, FormatType, StringsFormat, TSVFormat, XcstringsFormat,
    },
    placeholder::normalize_placeholders,
    traits::Parser,
    types::Resource,
};
use std::path::Path;

/// Convert a vector of resources to a specific output format.
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
/// use langcodec::{types::{Resource, Metadata, Entry, Translation, EntryStatus}, formats::FormatType, converter::convert_resources_to_format};
///
/// let resources = vec![Resource {
///     metadata: Metadata {
///         language: "en".to_string(),
///         domain: "domain".to_string(),
///         custom: std::collections::HashMap::new(),
///     },
///     entries: vec![],
/// }];
/// convert_resources_to_format(
///     resources,
///     "output.strings",
///     FormatType::Strings(None)
/// )?;
/// # Ok::<(), langcodec::Error>(())
/// ```
pub fn convert_resources_to_format(
    resources: Vec<Resource>,
    output_path: &str,
    output_format: FormatType,
) -> Result<(), Error> {
    match output_format {
        FormatType::AndroidStrings(_) => {
            if let Some(resource) = resources.into_iter().next() {
                AndroidStringsFormat::from(resource)
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
        FormatType::Strings(_) => {
            if let Some(resource) = resources.into_iter().next() {
                StringsFormat::try_from(resource)
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
        FormatType::Xcstrings => XcstringsFormat::try_from(resources)
            .and_then(|f| f.write_to(Path::new(output_path)))
            .map_err(|e| {
                Error::conversion_error(format!("Error writing Xcstrings output: {}", e), None)
            }),
        FormatType::CSV => CSVFormat::try_from(resources)
            .and_then(|f| f.write_to(Path::new(output_path)))
            .map_err(|e| Error::conversion_error(format!("Error writing CSV output: {}", e), None)),
        FormatType::TSV => TSVFormat::try_from(resources)
            .and_then(|f| f.write_to(Path::new(output_path)))
            .map_err(|e| Error::conversion_error(format!("Error writing TSV output: {}", e), None)),
    }
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
/// use langcodec::{converter::convert, formats::FormatType};
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
    let mut resources = match input_format {
        FormatType::AndroidStrings(_) => vec![AndroidStringsFormat::read_from(input)?.into()],
        FormatType::Strings(_) => vec![StringsFormat::read_from(input)?.into()],
        FormatType::Xcstrings => Vec::<Resource>::try_from(XcstringsFormat::read_from(input)?)?,
        FormatType::CSV => Vec::<Resource>::try_from(CSVFormat::read_from(input)?)?,
        FormatType::TSV => Vec::<Resource>::try_from(TSVFormat::read_from(input)?)?,
    };

    // Ensure language is set for single-language inputs if provided on input_format
    if let Some(l) = input_format.language().cloned() {
        for res in &mut resources {
            if res.metadata.language.is_empty() {
                res.metadata.language = l.clone();
            }
        }
    }

    // Helper to extract resource by language if present, or first one
    let pick_resource = |lang: Option<String>| -> Option<Resource> {
        match lang {
            Some(l) => resources.iter().find(|r| r.metadata.language == l).cloned(),
            None => resources.first().cloned(),
        }
    };

    match output_format {
        FormatType::AndroidStrings(lang) => {
            let resource = pick_resource(lang);
            if let Some(res) = resource {
                AndroidStringsFormat::from(res).write_to(output)
            } else {
                Err(Error::InvalidResource(
                    "No matching resource for output language.".to_string(),
                ))
            }
        }
        FormatType::Strings(lang) => {
            let resource = pick_resource(lang);
            if let Some(res) = resource {
                StringsFormat::try_from(res)?.write_to(output)
            } else {
                Err(Error::InvalidResource(
                    "No matching resource for output language.".to_string(),
                ))
            }
        }
        FormatType::Xcstrings => XcstringsFormat::try_from(resources)?.write_to(output),
        FormatType::CSV => CSVFormat::try_from(resources)?.write_to(output),
        FormatType::TSV => TSVFormat::try_from(resources)?.write_to(output),
    }
}

/// Convert like [`convert`], with an option to normalize placeholders before writing.
///
/// When `normalize` is true, common iOS placeholder tokens like `%@`, `%1$@`, `%ld` are
/// converted to canonical forms (`%s`, `%1$s`, `%d`) prior to serialization.
/// Convert with optional placeholder normalization.
///
/// Example
/// ```rust,no_run
/// use langcodec::formats::FormatType;
/// use langcodec::converter::convert_with_normalization;
/// convert_with_normalization(
///     "en.lproj/Localizable.strings",
///     FormatType::Strings(Some("en".to_string())),
///     "values/strings.xml",
///     FormatType::AndroidStrings(Some("en".to_string())),
///     true, // normalize placeholders (e.g., %@ -> %s)
/// )?;
/// # Ok::<(), langcodec::Error>(())
/// ```
pub fn convert_with_normalization<P: AsRef<Path>>(
    input: P,
    input_format: FormatType,
    output: P,
    output_format: FormatType,
    normalize: bool,
) -> Result<(), Error> {
    let input = input.as_ref();
    let output = output.as_ref();

    // Carry language between single-language formats
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
    let mut resources = match input_format {
        FormatType::AndroidStrings(_) => vec![AndroidStringsFormat::read_from(input)?.into()],
        FormatType::Strings(_) => vec![StringsFormat::read_from(input)?.into()],
        FormatType::Xcstrings => Vec::<Resource>::try_from(XcstringsFormat::read_from(input)?)?,
        FormatType::CSV => Vec::<Resource>::try_from(CSVFormat::read_from(input)?)?,
        FormatType::TSV => Vec::<Resource>::try_from(TSVFormat::read_from(input)?)?,
    };

    // Ensure language is set for single-language inputs if provided on input_format
    if let Some(l) = input_format.language().cloned() {
        for res in &mut resources {
            if res.metadata.language.is_empty() {
                res.metadata.language = l.clone();
            }
        }
    }

    if normalize {
        for res in &mut resources {
            for entry in &mut res.entries {
                match &mut entry.value {
                    crate::types::Translation::Empty => {
                        continue;
                    }
                    crate::types::Translation::Singular(v) => {
                        *v = normalize_placeholders(v);
                    }
                    crate::types::Translation::Plural(p) => {
                        for (_c, v) in p.forms.iter_mut() {
                            *v = normalize_placeholders(v);
                        }
                    }
                }
            }
        }
    }

    // Helper to extract resource by language if present, or first one
    let pick_resource = |lang: Option<String>| -> Option<Resource> {
        match lang {
            Some(l) => resources.iter().find(|r| r.metadata.language == l).cloned(),
            None => resources.first().cloned(),
        }
    };

    match output_format {
        FormatType::AndroidStrings(lang) => {
            let resource = pick_resource(lang);
            if let Some(res) = resource {
                AndroidStringsFormat::from(res).write_to(output)
            } else {
                Err(Error::InvalidResource(
                    "No matching resource for output language.".to_string(),
                ))
            }
        }
        FormatType::Strings(lang) => {
            let resource = pick_resource(lang);
            if let Some(res) = resource {
                StringsFormat::try_from(res)?.write_to(output)
            } else {
                Err(Error::InvalidResource(
                    "No matching resource for output language.".to_string(),
                ))
            }
        }
        FormatType::Xcstrings => XcstringsFormat::try_from(resources)?.write_to(output),
        FormatType::CSV => CSVFormat::try_from(resources)?.write_to(output),
        FormatType::TSV => TSVFormat::try_from(resources)?.write_to(output),
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
/// use langcodec::converter::convert_auto;
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
mod normalize_tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_convert_strings_to_android_with_normalization() {
        let tmp = tempfile::tempdir().unwrap();
        let strings = tmp.path().join("en.strings");
        let xml = tmp.path().join("strings.xml");

        fs::write(&strings, "\n\"g\" = \"Hello %@ and %1$@ and %ld\";\n").unwrap();

        // Without normalization: convert should succeed
        convert(
            &strings,
            FormatType::Strings(Some("en".into())),
            &xml,
            FormatType::AndroidStrings(Some("en".into())),
        )
        .unwrap();
        let content = fs::read_to_string(&xml).unwrap();
        assert!(content.contains("Hello %"));

        // With normalization
        convert_with_normalization(
            &strings,
            FormatType::Strings(Some("en".into())),
            &xml,
            FormatType::AndroidStrings(Some("en".into())),
            true,
        )
        .unwrap();
        let content = fs::read_to_string(&xml).unwrap();
        assert!(content.contains("%s"));
        assert!(content.contains("%1$s"));
        assert!(content.contains("%d"));
    }
}

/// Auto-infer formats from paths and convert, with optional placeholder normalization.
/// Auto-infer formats and convert with optional placeholder normalization.
///
/// Example
/// ```rust,no_run
/// use langcodec::converter::convert_auto_with_normalization;
/// convert_auto_with_normalization(
///     "Localizable.strings",
///     "strings.xml",
///     true, // normalize placeholders
/// )?;
/// # Ok::<(), langcodec::Error>(())
/// ```
pub fn convert_auto_with_normalization<P: AsRef<Path>>(
    input: P,
    output: P,
    normalize: bool,
) -> Result<(), Error> {
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
    convert_with_normalization(input, input_format, output, output_format, normalize)
}

/// Infers a [`FormatType`] from a file path's extension.
///
/// Returns `Some(FormatType)` if the extension matches a known format, otherwise `None`.
///
/// # Example
/// ```rust
/// use langcodec::formats::FormatType;
/// use langcodec::converter::infer_format_from_extension;
///
/// assert_eq!(
///     infer_format_from_extension("Localizable.strings"),
///     Some(FormatType::Strings(None))
/// );
/// assert_eq!(
///     infer_format_from_extension("strings.xml"),
///     Some(FormatType::AndroidStrings(None))
/// );
/// assert_eq!(
///     infer_format_from_extension("Localizable.xcstrings"),
///     Some(FormatType::Xcstrings)
/// );
/// assert_eq!(
///     infer_format_from_extension("translations.csv"),
///     Some(FormatType::CSV)
/// );
/// assert_eq!(
///     infer_format_from_extension("data.tsv"),
///     Some(FormatType::TSV)
/// );
/// assert_eq!(
///     infer_format_from_extension("unknown.xyz"),
///     None
/// );
/// ```
pub fn infer_format_from_extension<P: AsRef<Path>>(path: P) -> Option<FormatType> {
    let path = path.as_ref();
    let extension = path.extension()?.to_str()?;

    match extension.to_lowercase().as_str() {
        "strings" => Some(FormatType::Strings(None)),
        "xml" => Some(FormatType::AndroidStrings(None)),
        "xcstrings" => Some(FormatType::Xcstrings),
        "csv" => Some(FormatType::CSV),
        "tsv" => Some(FormatType::TSV),
        _ => None,
    }
}

/// Infers a [`FormatType`] from a file path, including language detection.
///
/// This function combines extension-based format detection with language inference
/// from the path structure (e.g., `values-es/strings.xml` → Spanish Android strings).
///
/// # Example
/// ```rust
/// use langcodec::formats::FormatType;
/// use langcodec::converter::infer_format_from_path;
///
/// assert_eq!(
///     infer_format_from_path("en.lproj/Localizable.strings"),
///     Some(FormatType::Strings(Some("en".to_string())))
/// );
/// assert_eq!(
///     infer_format_from_path("zh-Hans.lproj/Localizable.strings"),
///     Some(FormatType::Strings(Some("zh-Hans".to_string())))
/// );
/// assert_eq!(
///     infer_format_from_path("values-es/strings.xml"),
///     Some(FormatType::AndroidStrings(Some("es".to_string())))
/// );
/// assert_eq!(
///     infer_format_from_path("values/strings.xml"),
///     Some(FormatType::AndroidStrings(Some("en".to_string())))
/// );
/// assert_eq!(
///     infer_format_from_path("Localizable.xcstrings"),
///     Some(FormatType::Xcstrings)
/// );
/// ```
pub fn infer_format_from_path<P: AsRef<Path>>(path: P) -> Option<FormatType> {
    match infer_format_from_extension(&path) {
        Some(format) => match format {
            // Multi-language formats, no language inference needed
            FormatType::Xcstrings | FormatType::CSV | FormatType::TSV => Some(format),
            FormatType::AndroidStrings(_) | FormatType::Strings(_) => {
                let lang = infer_language_from_path(&path, &format).ok().flatten();
                Some(format.with_language(lang))
            }
        },
        None => None,
    }
}

/// Infers the language code from a file path based on its format and structure.
///
/// This function analyzes the path to extract language information based on common
/// localization file naming conventions.
///
/// # Arguments
///
/// * `path` - The file path to analyze
/// * `format` - The format type to help with language inference
///
/// # Returns
///
/// `Ok(Some(language_code))` if a language can be inferred, `Ok(None)` if no language
/// can be determined, or `Err` if there's an error in the inference process.
///
/// # Example
///
/// ```rust
/// use langcodec::{converter::infer_language_from_path, formats::FormatType};
/// use std::path::Path;
///
/// // Apple .strings files
/// assert_eq!(
///     infer_language_from_path("en.lproj/Localizable.strings", &FormatType::Strings(None)).unwrap(),
///     Some("en".to_string())
/// );
/// assert_eq!(
///     infer_language_from_path("fr.lproj/Localizable.strings", &FormatType::Strings(None)).unwrap(),
///     Some("fr".to_string())
/// );
/// assert_eq!(
///     infer_language_from_path("zh-Hans.lproj/Localizable.strings", &FormatType::Strings(None)).unwrap(),
///     Some("zh-Hans".to_string())
/// );
///
/// // Android strings.xml files
/// assert_eq!(
///     infer_language_from_path("values-es/strings.xml", &FormatType::AndroidStrings(None)).unwrap(),
///     Some("es".to_string())
/// );
/// assert_eq!(
///     infer_language_from_path("values-fr/strings.xml", &FormatType::AndroidStrings(None)).unwrap(),
///     Some("fr".to_string())
/// );
///
/// // No language in path
/// assert_eq!(
///     infer_language_from_path("values/strings.xml", &FormatType::AndroidStrings(None)).unwrap(),
///     Some("en".to_string())
/// );
/// ```
pub fn infer_language_from_path<P: AsRef<Path>>(
    path: P,
    format: &FormatType,
) -> Result<Option<String>, Error> {
    use std::str::FromStr;
    use unic_langid::LanguageIdentifier;

    let path = path.as_ref();

    // Helper: validate and normalize a language candidate (accepts underscores, normalizes to hyphens)
    fn normalize_lang(candidate: &str) -> Option<String> {
        let canonical = candidate.replace('_', "-");
        LanguageIdentifier::from_str(&canonical).ok()?;
        Some(canonical)
    }

    // Helper: parse Android values- qualifiers into BCP-47 if possible
    fn parse_android_values_lang(values_component: &str) -> Option<String> {
        // values-zh-rCN → zh-CN; values-es → es; values-b+zh+Hans+CN → zh-Hans-CN
        if let Some(rest) = values_component.strip_prefix("values-") {
            if rest.is_empty() {
                return None;
            }
            if let Some(b_rest) = rest.strip_prefix("b+") {
                // BCP-47 style encoded in plus-separated tags
                let parts: Vec<&str> = b_rest.split('+').collect();
                if parts.is_empty() {
                    return None;
                }
                let lang = parts.join("-");
                return normalize_lang(&lang);
            }
            // Legacy qualifiers: lang[-rREGION][-SCRIPT]...
            let mut lang: Option<String> = None;
            let mut region: Option<String> = None;
            for token in rest.split('-') {
                if token.is_empty() {
                    continue;
                }
                if let Some(r) = token.strip_prefix('r') {
                    if !r.is_empty() {
                        region = Some(r.to_string());
                    }
                } else if lang.is_none() {
                    lang = Some(token.to_string());
                }
            }
            if let Some(l) = lang {
                let mut tag = l;
                if let Some(r) = region {
                    tag = format!("{}-{}", tag, r);
                }
                return normalize_lang(&tag);
            }
        }
        None
    }

    // Iterate from the filename upward until a language is found
    let mut components: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    components.reverse();

    for comp in components {
        match format {
            FormatType::Strings(_) => {
                // Apple: directory like zh-Hans.lproj, or filename like en.strings
                if let Some(lang_dir) = comp.strip_suffix(".lproj")
                    && let Some(lang) = normalize_lang(lang_dir)
                {
                    return Ok(Some(lang));
                }
                if comp.ends_with(".strings")
                    && let Some(stem) = Path::new(&comp).file_stem().and_then(|s| s.to_str())
                {
                    let looks_like_lang = (stem.len() == 2
                        && stem.chars().all(|c| c.is_ascii_lowercase()))
                        || stem.contains('-');
                    if looks_like_lang && let Some(lang) = normalize_lang(stem) {
                        return Ok(Some(lang));
                    }
                }
            }
            FormatType::AndroidStrings(_) => {
                // Android: values (default → en), values-xx, values-xx-rYY, values-b+zh+Hans+CN, etc.
                if comp == "values" {
                    // Treat base `values/` as English by default
                    return Ok(Some("en".to_string()));
                }
                if let Some(lang) = parse_android_values_lang(&comp) {
                    return Ok(Some(lang));
                }
            }
            _ => {}
        }
    }

    Ok(None)
}

/// Writes resources to a file based on their stored format metadata.
///
/// This function determines the output format from the resource metadata and writes
/// the resources accordingly. Supports formats with single or multiple resources per file.
///
/// # Parameters
/// - `resources`: Slice of resources to write.
/// - `file_path`: Destination file path.
///
/// # Returns
///
/// `Ok(())` if writing succeeds, or an `Error` if the format is unsupported or writing fails.
pub fn write_resources_to_file(resources: &[Resource], file_path: &String) -> Result<(), Error> {
    let path = Path::new(&file_path);

    if let Some(first) = resources.first() {
        match first.metadata.custom.get("format").map(String::as_str) {
            Some("AndroidStrings") => AndroidStringsFormat::from(first.clone()).write_to(path)?,
            Some("Strings") => StringsFormat::try_from(first.clone())?.write_to(path)?,
            Some("Xcstrings") => XcstringsFormat::try_from(resources.to_vec())?.write_to(path)?,
            Some("CSV") => CSVFormat::try_from(resources.to_vec())?.write_to(path)?,
            Some("TSV") => TSVFormat::try_from(resources.to_vec())?.write_to(path)?,
            _ => Err(Error::UnsupportedFormat(format!(
                "Unsupported format: {:?}",
                first.metadata.custom.get("format")
            )))?,
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
/// use langcodec::{converter::merge_resources, types::{Resource, Metadata, Entry, Translation, EntryStatus, ConflictStrategy}};
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
/// let merged = merge_resources(
///     &[resource1],
///     &ConflictStrategy::Last
/// )?;
/// # Ok::<(), langcodec::Error>(())
/// ```
pub fn merge_resources(
    resources: &[Resource],
    conflict_strategy: &ConflictStrategy,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Entry, EntryStatus, Metadata, Plural, PluralCategory, Translation};
    use std::collections::{BTreeMap, HashMap};

    #[test]
    fn test_convert_csv_to_android_strings_en() {
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("in.csv");
        let output = tmp.path().join("strings.xml");

        // CSV with header and two rows
        std::fs::write(
            &input,
            "key,en,fr\nhello,Hello,Bonjour\nbye,Goodbye,Au revoir\n",
        )
        .unwrap();

        convert(
            &input,
            FormatType::CSV,
            &output,
            FormatType::AndroidStrings(Some("en".into())),
        )
        .unwrap();

        // Read back as Android to verify
        let android = crate::formats::AndroidStringsFormat::read_from(&output).unwrap();
        // ensure we have only strings for the selected language
        assert_eq!(android.strings.len(), 2);
        let mut names: Vec<&str> = android.strings.iter().map(|s| s.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["bye", "hello"]);
        let hello = android.strings.iter().find(|s| s.name == "hello").unwrap();
        assert_eq!(hello.value, "Hello");
        let bye = android.strings.iter().find(|s| s.name == "bye").unwrap();
        assert_eq!(bye.value, "Goodbye");
    }

    #[test]
    fn test_convert_xcstrings_plurals_to_android() {
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("in.xcstrings");
        let output = tmp.path().join("strings.xml");

        // Build a Resource with a plural entry for English
        let mut custom = HashMap::new();
        custom.insert("source_language".into(), "en".into());
        custom.insert("version".into(), "1.0".into());

        let mut forms = BTreeMap::new();
        forms.insert(PluralCategory::One, "One apple".to_string());
        forms.insert(PluralCategory::Other, "%d apples".to_string());

        let res = Resource {
            metadata: Metadata {
                language: "en".into(),
                domain: "domain".into(),
                custom,
            },
            entries: vec![Entry {
                id: "apples".into(),
                value: Translation::Plural(Plural {
                    id: "apples".into(),
                    forms,
                }),
                comment: Some("Count apples".into()),
                status: EntryStatus::Translated,
                custom: HashMap::new(),
            }],
        };

        // Write XCStrings input
        let xc = crate::formats::XcstringsFormat::try_from(vec![res]).unwrap();
        xc.write_to(&input).unwrap();

        // Convert to Android (English)
        convert(
            &input,
            FormatType::Xcstrings,
            &output,
            FormatType::AndroidStrings(Some("en".into())),
        )
        .unwrap();

        // Read back as Android
        let android = crate::formats::AndroidStringsFormat::read_from(&output).unwrap();
        assert_eq!(android.plurals.len(), 1);
        let p = android.plurals.into_iter().next().unwrap();
        assert_eq!(p.name, "apples");
        // Should include at least 'one' and 'other'
        let mut qs: Vec<_> = p
            .items
            .into_iter()
            .map(|i| match i.quantity {
                PluralCategory::One => ("one", i.value),
                PluralCategory::Other => ("other", i.value),
                PluralCategory::Zero => ("zero", i.value),
                PluralCategory::Two => ("two", i.value),
                PluralCategory::Few => ("few", i.value),
                PluralCategory::Many => ("many", i.value),
            })
            .collect();
        qs.sort_by(|a, b| a.0.cmp(b.0));
        assert!(qs.iter().any(|(q, v)| *q == "one" && v == "One apple"));
        assert!(qs.iter().any(|(q, v)| *q == "other" && v == "%d apples"));
    }
}
