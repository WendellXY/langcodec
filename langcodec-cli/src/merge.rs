use crate::formats::parse_custom_format;
use crate::transformers::custom_format_to_resource;

use langcodec::{Codec, converter};

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

    // Read all input files into a single codec
    let mut codec = Codec::new();
    for (i, input) in inputs.iter().enumerate() {
        println!("Reading file {}/{}: {}", i + 1, inputs.len(), input);

        // Try standard format first
        if let Ok(()) = codec.read_file_by_extension(input, lang.clone()) {
            continue;
        }

        // If standard format fails, try custom format for JSON/YAML files
        if (input.ends_with(".json") || input.ends_with(".yaml") || input.ends_with(".yml"))
            && let Ok(()) = try_custom_format_merge(input, lang.clone(), &mut codec)
        {
            continue;
        }

        // If both fail, show error
        println!("❌ Error reading input file");
        eprintln!("Error reading {}: unsupported format", input);
        std::process::exit(1);
    }

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
            let source_language = codec
                .resources
                .first()
                .and_then(|r| r.metadata.custom.get("source_language").cloned())
                .unwrap_or_else(|| {
                    codec
                        .resources
                        .first()
                        .map(|r| r.metadata.language.clone())
                        .unwrap_or("en".to_string())
                });

            println!("Setting metadata.source_language to: {}", source_language);

            // Set version field in the resources to make sure xcstrings format would not throw an error
            let version = codec
                .resources
                .first()
                .and_then(|r| r.metadata.custom.get("version").cloned())
                .unwrap_or_else(|| "1.0".to_string());

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

/// Try to read a custom format file and add it to the codec
fn try_custom_format_merge(
    input: &str,
    _lang: Option<String>,
    codec: &mut Codec,
) -> Result<(), String> {
    // Validate custom format file
    crate::validation::validate_custom_format_file(input)?;

    // Auto-detect format based on file content
    let file_content = std::fs::read_to_string(input)
        .map_err(|e| format!("Error reading file {}: {}", input, e))?;

    // Validate file content
    crate::formats::validate_custom_format_content(input, &file_content)?;

    // Convert custom format to Resource
    let resources =
        custom_format_to_resource(input.to_string(), parse_custom_format("json-language-map")?)?;

    // Add resources to codec
    for resource in resources {
        codec.add_resource(resource);
    }

    Ok(())
}
