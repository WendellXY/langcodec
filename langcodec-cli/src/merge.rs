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

    // Infer format from the first input
    let first_format = langcodec::infer_format_from_extension(&inputs[0])
        .or_else(|| langcodec::codec::infer_format_from_path(&inputs[0]))
        .unwrap_or_else(|| {
            eprintln!("Error: Cannot infer format from extension: {}", &inputs[0]);
            std::process::exit(1);
        });

    // Read all input files into a single codec
    let mut codec = Codec::new();
    for input in &inputs {
        if let Err(e) = codec.read_file_by_extension(input, lang.clone()) {
            eprintln!("Error reading {}: {}", input, e);
            std::process::exit(1);
        }
    }

    // Merge resources directly in the codec
    let merged = merge_resources_cli(&codec.resources, strategy);
    if let Err(e) = merged {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    let merged_resources = merged.unwrap();

    // Write merged resources to output file
    match first_format {
        langcodec::formats::FormatType::AndroidStrings(_) => {
            use langcodec::formats::AndroidStringsFormat;
            if let Err(e) = AndroidStringsFormat::from(merged_resources).write_to(&output) {
                eprintln!("Error writing output: {}", e);
                std::process::exit(1);
            }
        }
        langcodec::formats::FormatType::Strings(_) => {
            use langcodec::formats::StringsFormat;
            if let Err(e) =
                StringsFormat::try_from(merged_resources).and_then(|f| f.write_to(&output))
            {
                eprintln!("Error writing output: {}", e);
                std::process::exit(1);
            }
        }
        langcodec::formats::FormatType::Xcstrings => {
            use langcodec::formats::XcstringsFormat;
            if let Err(e) =
                XcstringsFormat::try_from(vec![merged_resources]).and_then(|f| f.write_to(&output))
            {
                eprintln!("Error writing output: {}", e);
                std::process::exit(1);
            }
        }
    }
}

// Merge multiple resources into a single resource, handling conflicts.
fn merge_resources_cli(
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
