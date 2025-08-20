use crate::formats::parse_custom_format;
use crate::transformers::custom_format_to_resource;

use langcodec::Codec;

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
        if input.ends_with(".json") || input.ends_with(".yaml") || input.ends_with(".yml") {
            if let Ok(()) = try_custom_format_merge(input, lang.clone(), &mut codec) {
                continue;
            }
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

    let merged_resource = match Codec::merge_resources(&codec.resources, conflict_strategy) {
        Ok(resource) => resource,
        Err(e) => {
            println!("❌ Error merging resources");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Write merged resource to output file using the new lib crate method
    println!("Writing merged output...");
    if let Err(e) = Codec::write_resource_to_file(&merged_resource, &output) {
        println!("❌ Error writing output file");
        eprintln!("Error writing to {}: {}", output, e);
        std::process::exit(1);
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
