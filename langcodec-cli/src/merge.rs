use indicatif::{ProgressBar, ProgressStyle};
use langcodec::Codec;
use langcodec::traits::Parser as CodecParser;

/// Strategy for handling conflicts when merging localization files.
#[derive(Debug, Clone, PartialEq, clap::ValueEnum)]
pub enum ConflictStrategy {
    /// Keep the first occurrence of a key
    First,
    /// Keep the last occurrence of a key (default)
    Last,
    /// Skip conflicting entries
    Skip,
}

/// Run the merge command: merge multiple localization files into one output file.
pub fn run_merge_command(
    inputs: Vec<String>,
    output: String,
    strategy: ConflictStrategy,
    lang: Option<String>,
) {
    if inputs.is_empty() {
        eprintln!("Error: At least one input file is required.");
        std::process::exit(1);
    }

    // Create progress bar
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    // Read all input files into a single codec
    let mut codec = Codec::new();
    for (i, input) in inputs.iter().enumerate() {
        progress_bar.set_message(format!(
            "Reading file {}/{}: {}",
            i + 1,
            inputs.len(),
            input
        ));
        if let Err(e) = codec.read_file_by_extension(input, lang.clone()) {
            progress_bar.finish_with_message("❌ Error reading input file");
            eprintln!("Error reading {}: {}", input, e);
            std::process::exit(1);
        }
    }

    // Validate that all resources have the same format
    progress_bar.set_message("Validating format consistency...");
    if let Err(e) = codec.validate() {
        progress_bar.finish_with_message("❌ Validation failed");
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Merge resources using the enhanced lib crate methods
    progress_bar.set_message("Merging resources...");
    let merged_resource = merge_resources_enhanced(&codec.resources, strategy);
    if let Err(e) = merged_resource {
        progress_bar.finish_with_message("❌ Error merging resources");
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    let merged_resource = merged_resource.unwrap();

    // Write merged resource to output file
    progress_bar.set_message("Writing merged output...");
    if let Err(e) = write_merged_resource_to_file(&merged_resource, &output) {
        progress_bar.finish_with_message("❌ Error writing output file");
        eprintln!("Error writing to {}: {}", output, e);
        std::process::exit(1);
    }

    progress_bar.finish_with_message(format!(
        "✅ Successfully merged {} files into {}",
        inputs.len(),
        output
    ));
}

// Enhanced merge function that leverages the new lib crate capabilities
fn merge_resources_enhanced(
    resources: &[langcodec::Resource],
    conflict_strategy: ConflictStrategy,
) -> Result<langcodec::Resource, String> {
    if resources.is_empty() {
        return Err("No resources to merge.".to_string());
    }

    let mut merged = resources[0].clone();
    let mut all_entries = std::collections::HashMap::new();

    // Collect all entries from all resources
    for resource in resources {
        for entry in &resource.entries {
            let key = entry.id.clone();
            match conflict_strategy {
                ConflictStrategy::First => {
                    all_entries.entry(key).or_insert_with(|| entry.clone());
                }
                ConflictStrategy::Last => {
                    all_entries.insert(key, entry.clone());
                }
                ConflictStrategy::Skip => {
                    if all_entries.contains_key(&key) {
                        // Skip this entry if we already have one with the same key
                        continue;
                    }
                    all_entries.insert(key, entry.clone());
                }
            }
        }
    }

    // Convert back to vector and sort by key for consistent output
    merged.entries = all_entries.into_values().collect();
    merged.entries.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(merged)
}

// Simplified write function that uses the lib crate's format detection
fn write_merged_resource_to_file(
    merged_resource: &langcodec::Resource,
    output_path: &str,
) -> Result<(), String> {
    use langcodec::formats::{AndroidStringsFormat, CSVRecord, StringsFormat, XcstringsFormat};
    use std::path::Path;

    // Infer format from output path
    let format_type = langcodec::infer_format_from_extension(output_path)
        .ok_or_else(|| format!("Cannot infer format from output path: {}", output_path))?;

    match format_type {
        langcodec::formats::FormatType::AndroidStrings(_) => {
            AndroidStringsFormat::from(merged_resource.clone())
                .write_to(Path::new(output_path))
                .map_err(|e| format!("Error writing AndroidStrings output: {}", e))
        }
        langcodec::formats::FormatType::Strings(_) => {
            StringsFormat::try_from(merged_resource.clone())
                .and_then(|f| f.write_to(Path::new(output_path)))
                .map_err(|e| format!("Error writing Strings output: {}", e))
        }
        langcodec::formats::FormatType::Xcstrings => {
            XcstringsFormat::try_from(vec![merged_resource.clone()])
                .and_then(|f| f.write_to(Path::new(output_path)))
                .map_err(|e| format!("Error writing Xcstrings output: {}", e))
        }
        langcodec::formats::FormatType::CSV(_) => {
            Vec::<CSVRecord>::try_from(merged_resource.clone())
                .and_then(|f| f.write_to(Path::new(output_path)))
                .map_err(|e| format!("Error writing CSV output: {}", e))
        }
    }
}
