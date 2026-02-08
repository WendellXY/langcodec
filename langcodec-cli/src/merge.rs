use crate::formats::parse_custom_format;
use crate::transformers::custom_format_to_resource;

use langcodec::{Codec, converter};
use rayon::prelude::*;

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
    source_language_override: Option<String>,
    version_override: Option<String>,
    strict: bool,
) {
    if inputs.is_empty() {
        eprintln!("Error: At least one input file is required.");
        std::process::exit(1);
    }

    // Read all input files concurrently into Codecs, then combine and merge
    println!("Reading {} input files...", inputs.len());
    let read_results: Vec<Result<Codec, String>> = inputs
        .par_iter()
        .map(|input| read_input_to_codec(input, lang.clone(), strict))
        .collect();

    let mut input_codecs: Vec<Codec> = Vec::with_capacity(read_results.len());
    for (idx, res) in read_results.into_iter().enumerate() {
        match res {
            Ok(c) => input_codecs.push(c),
            Err(e) => {
                println!("❌ Error reading input file {}/{}", idx + 1, inputs.len());
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }

    // Combine all input codecs first, then merge by language
    let mut codec = Codec::from_codecs(input_codecs);

    // Skip validation for merge operations since we expect multiple resources with potentially duplicate languages

    // Merge resources using the new lib crate method
    println!("Merging resources...");
    let conflict_strategy = match strategy {
        ConflictStrategy::First => langcodec::types::ConflictStrategy::First,
        ConflictStrategy::Last => langcodec::types::ConflictStrategy::Last,
        ConflictStrategy::Skip => langcodec::types::ConflictStrategy::Skip,
    };

    let merge_count = codec.merge_resources(&conflict_strategy);
    println!("Merged {} language groups", merge_count);

    println!("Writing merged output...");
    match converter::infer_format_from_path(output.clone()) {
        Some(format) => {
            println!("Converting resources to format: {:?}", format);
            // Set source_language field in the resources to make sure xcstrings format would not throw an error
            // First, try to get the source language from the first resource if it exists; otherwise, the first resource's language
            // would be used as the source language. If the two checks fail, the default value "en" would be used.
            let source_language = source_language_override
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| {
                    codec
                        .resources
                        .first()
                        .and_then(|r| {
                            r.metadata
                                .custom
                                .get("source_language")
                                .cloned()
                                .filter(|s| !s.trim().is_empty())
                        })
                        .unwrap_or_else(|| {
                            codec
                                .resources
                                .first()
                                .map(|r| r.metadata.language.clone())
                                .unwrap_or("en".to_string())
                        })
                });

            println!("Setting metadata.source_language to: {}", source_language);

            // Set version field in the resources to make sure xcstrings format would not throw an error
            let version = version_override.unwrap_or_else(|| {
                codec
                    .resources
                    .first()
                    .and_then(|r| r.metadata.custom.get("version").cloned())
                    .unwrap_or_else(|| "1.0".to_string())
            });

            println!("Setting metadata.version to: {}", version);

            codec.iter_mut().for_each(|r| {
                r.metadata
                    .custom
                    .insert("source_language".to_string(), source_language.clone());
                r.metadata
                    .custom
                    .insert("version".to_string(), version.clone());
            });

            if let Err(e) = converter::convert_resources_to_format(codec.resources, &output, format)
            {
                println!("❌ Error converting resources to format");
                eprintln!("Error converting to {}: {}", output, e);
                std::process::exit(1);
            }
        }
        None => {
            if codec.resources.len() == 1 {
                println!("Writing single resource to output file");
                if let Some(resource) = codec.resources.first()
                    && let Err(e) = Codec::write_resource_to_file(resource, &output)
                {
                    println!("❌ Error writing output file");
                    eprintln!("Error writing to {}: {}", output, e);
                    std::process::exit(1);
                }
            } else {
                println!("❌ Error writing output file");
                eprintln!("Error writing to {}: multiple resources", output);
                std::process::exit(1);
            }
        }
    }

    println!(
        "✅ Successfully merged {} files into {}",
        inputs.len(),
        output
    );
}

/// Read a single input file into a vector of Resources, supporting both standard and custom formats
fn read_input_to_resources(
    input: &str,
    lang: Option<String>,
    strict: bool,
) -> Result<Vec<langcodec::Resource>, String> {
    if strict {
        if input.ends_with(".json") || input.ends_with(".yaml") || input.ends_with(".yml") {
            crate::validation::validate_custom_format_file(input)
                .map_err(|e| format!("Failed to validate {}: {}", input, e))?;

            let file_content = std::fs::read_to_string(input)
                .map_err(|e| format!("Error reading file {}: {}", input, e))?;

            crate::formats::validate_custom_format_content(input, &file_content)
                .map_err(|e| format!("Invalid custom format {}: {}", input, e))?;

            let resources = custom_format_to_resource(
                input.to_string(),
                parse_custom_format("json-language-map")
                    .map_err(|e| format!("Failed to parse custom format: {}", e))?,
            )
            .map_err(|e| format!("Failed to convert custom format {}: {}", input, e))?;

            return Ok(resources);
        }

        let mut local_codec = Codec::new();
        local_codec
            .read_file_by_extension(input, lang)
            .map_err(|e| format!("Error reading {}: {}", input, e))?;
        return Ok(local_codec.resources);
    }

    // Try standard format via lib crate (uses extension + language inference)
    {
        let mut local_codec = Codec::new();
        if let Ok(()) = local_codec.read_file_by_extension(input, lang.clone()) {
            return Ok(local_codec.resources);
        }
    }

    // Try custom JSON/YAML formats (for merge, we follow the existing JSON-language-map behavior)
    if input.ends_with(".json") || input.ends_with(".yaml") || input.ends_with(".yml") {
        // Validate custom format file
        crate::validation::validate_custom_format_file(input)
            .map_err(|e| format!("Failed to validate {}: {}", input, e))?;

        // Auto-detect format based on file content
        let file_content = std::fs::read_to_string(input)
            .map_err(|e| format!("Error reading file {}: {}", input, e))?;

        // Validate file content (ignore returned format; keep parity with existing merge behavior)
        crate::formats::validate_custom_format_content(input, &file_content)
            .map_err(|e| format!("Invalid custom format {}: {}", input, e))?;

        // Convert custom format to Resource using JSON language map to match current merge behavior
        let resources = custom_format_to_resource(
            input.to_string(),
            parse_custom_format("json-language-map")
                .map_err(|e| format!("Failed to parse custom format: {}", e))?,
        )
        .map_err(|e| format!("Failed to convert custom format {}: {}", input, e))?;

        return Ok(resources);
    }

    Err(format!("Error reading {}: unsupported format", input))
}

/// Read a single input into a Codec (wrapper over read_input_to_resources)
fn read_input_to_codec(input: &str, lang: Option<String>, strict: bool) -> Result<Codec, String> {
    let resources = read_input_to_resources(input, lang, strict)?;
    Ok(Codec { resources })
}
