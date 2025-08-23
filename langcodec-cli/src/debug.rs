use crate::formats::parse_custom_format;
use crate::transformers::custom_format_to_resource;

use langcodec::Codec;
use std::fs::File;
use std::io::Write;

/// Run the debug command: read a localization file and output as JSON.
pub fn run_debug_command(input: String, lang: Option<String>, output: Option<String>) {
    // Read the input file
    println!("Reading input file...");
    let mut codec = Codec::new();
    // Try standard format first
    if let Ok(()) = codec.read_file_by_extension(&input, lang.clone()) {
        // Standard format succeeded
    } else if input.ends_with(".json") || input.ends_with(".yaml") || input.ends_with(".yml") {
        // Try custom format for JSON/YAML files
        if let Err(e) = try_custom_format_debug(&input, lang.clone(), &mut codec) {
            println!("❌ Error reading input file");
            eprintln!("Error reading {}: {}", input, e);
            std::process::exit(1);
        }
    } else {
        println!("❌ Error reading input file");
        // Provide a hint about encoding issues for common Apple .strings files
        eprintln!(
            "Error reading {}: unsupported format or invalid text encoding",
            input
        );
        std::process::exit(1);
    }

    // Validate the codec using the new validation method
    println!("Validating resources...");
    if let Err(validation_error) = codec.validate() {
        println!("⚠️  Validation warnings found");
        eprintln!("Warning: {}", validation_error);
        // Continue anyway for debug purposes
    } else {
        println!("✅ Resources validated successfully");
    }

    // Convert to JSON
    println!("Converting to JSON...");
    let json = serde_json::to_string_pretty(&*codec.resources).unwrap_or_else(|e| {
        println!("❌ Error serializing to JSON");
        eprintln!("Error serializing to JSON: {}", e);
        std::process::exit(1);
    });

    // Output to file or stdout
    let output_to_file = match output {
        Some(output_path) => {
            println!("Writing output file...");
            if let Err(e) =
                File::create(&output_path).and_then(|mut f| f.write_all(json.as_bytes()))
            {
                println!("❌ Error writing output file");
                eprintln!("Error writing to {}: {}", output_path, e);
                std::process::exit(1);
            }
            println!("✅ Debug output written to: {}", output_path);
            true
        }
        None => {
            println!("✅ Debug output:");
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

/// Try to read a custom format file and add it to the codec for debug
fn try_custom_format_debug(
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
