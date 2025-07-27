use indicatif::{ProgressBar, ProgressStyle};
use langcodec::Codec;
use std::fs::File;
use std::io::Write;

/// Run the debug command: read a localization file and output as JSON.
pub fn run_debug_command(input: String, lang: Option<String>, output: Option<String>) {
    // Create progress bar
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    // Read the input file
    progress_bar.set_message("Reading input file...");
    let mut codec = Codec::new();
    if let Err(e) = codec.read_file_by_extension(&input, lang) {
        progress_bar.finish_with_message("❌ Error reading input file");
        eprintln!("Error reading {}: {}", input, e);
        std::process::exit(1);
    }

    // Validate the codec using the new validation method
    progress_bar.set_message("Validating resources...");
    if let Err(validation_error) = codec.validate() {
        progress_bar.finish_with_message("⚠️  Validation warnings found");
        eprintln!("Warning: {}", validation_error);
        // Continue anyway for debug purposes
    } else {
        progress_bar.set_message("✅ Resources validated successfully");
    }

    // Convert to JSON
    progress_bar.set_message("Converting to JSON...");
    let json = serde_json::to_string_pretty(&*codec.resources).unwrap_or_else(|e| {
        progress_bar.finish_with_message("❌ Error serializing to JSON");
        eprintln!("Error serializing to JSON: {}", e);
        std::process::exit(1);
    });

    // Output to file or stdout
    let output_to_file = match output {
        Some(output_path) => {
            progress_bar.set_message("Writing output file...");
            if let Err(e) =
                File::create(&output_path).and_then(|mut f| f.write_all(json.as_bytes()))
            {
                progress_bar.finish_with_message("❌ Error writing output file");
                eprintln!("Error writing to {}: {}", output_path, e);
                std::process::exit(1);
            }
            progress_bar
                .finish_with_message(format!("✅ Debug output written to: {}", output_path));
            true
        }
        None => {
            progress_bar.finish_with_message("✅ Debug output:");
            println!("{}", json);
            false
        }
    };

    // Show additional debug information using the new high-level methods
    if !output_to_file {
        println!("\n=== Debug Summary ===");
        println!(
            "Languages: {}",
            codec.languages().collect::<Vec<_>>().join(", ")
        );
        println!("Total entries: {}", codec.all_keys().count());

        for lang in codec.languages() {
            let count = codec.entry_count(lang);
            println!("  {}: {} entries", lang, count);
        }
    }
}
