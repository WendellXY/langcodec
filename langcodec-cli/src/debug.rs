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

    // Convert to JSON
    progress_bar.set_message("Converting to JSON...");
    let json = serde_json::to_string_pretty(&*codec.resources).unwrap_or_else(|e| {
        progress_bar.finish_with_message("❌ Error serializing to JSON");
        eprintln!("Error serializing to JSON: {}", e);
        std::process::exit(1);
    });

    // Output to file or stdout
    match output {
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
        }
        None => {
            progress_bar.finish_with_message("✅ Debug output:");
            println!("{}", json);
        }
    }
}
