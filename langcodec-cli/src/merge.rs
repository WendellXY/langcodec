use indicatif::{ProgressBar, ProgressStyle};
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

    // Merge resources using the new lib crate method
    progress_bar.set_message("Merging resources...");
    let conflict_strategy = match strategy {
        ConflictStrategy::First => langcodec::types::ConflictStrategy::First,
        ConflictStrategy::Last => langcodec::types::ConflictStrategy::Last,
        ConflictStrategy::Skip => langcodec::types::ConflictStrategy::Skip,
    };

    let merged_resource = match Codec::merge_resources(&codec.resources, conflict_strategy) {
        Ok(resource) => resource,
        Err(e) => {
            progress_bar.finish_with_message("❌ Error merging resources");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Write merged resource to output file using the new lib crate method
    progress_bar.set_message("Writing merged output...");
    if let Err(e) = Codec::write_resource_to_file(&merged_resource, &output) {
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
