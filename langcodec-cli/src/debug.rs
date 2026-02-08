use crate::formats::parse_custom_format;
use crate::transformers::custom_format_to_resource;

use langcodec::{Codec, Plural, Translation};
use std::fs::File;
use std::io::{self, Write};

/// Run the debug command: read a localization file and output as JSON.
pub fn run_debug_command(
    input: String,
    lang: Option<String>,
    output: Option<String>,
    strict: bool,
) {
    // Read the input file
    eprintln!("Reading input file...");
    let mut codec = Codec::new();
    let is_custom_ext =
        input.ends_with(".json") || input.ends_with(".yaml") || input.ends_with(".yml");
    if strict {
        if is_custom_ext {
            if let Err(e) = try_custom_format_debug(&input, lang.clone(), &mut codec) {
                eprintln!("❌ Error reading input file");
                eprintln!("Error reading {}: {}", input, e);
                std::process::exit(1);
            }
        } else if let Err(e) = codec.read_file_by_extension(&input, lang.clone()) {
            eprintln!("❌ Error reading input file");
            eprintln!("Error reading {}: {}", input, e);
            std::process::exit(1);
        }
    } else if let Ok(()) = codec.read_file_by_extension(&input, lang.clone()) {
        // Standard format succeeded
    } else if is_custom_ext {
        // Try custom format for JSON/YAML files
        if let Err(e) = try_custom_format_debug(&input, lang.clone(), &mut codec) {
            eprintln!("❌ Error reading input file");
            eprintln!("Error reading {}: {}", input, e);
            std::process::exit(1);
        }
    } else {
        eprintln!("❌ Error reading input file");
        // Provide a hint about encoding issues for common Apple .strings files
        eprintln!(
            "Error reading {}: unsupported format or invalid text encoding",
            input
        );
        std::process::exit(1);
    }

    // Validate the codec using the new validation method
    eprintln!("Validating resources...");
    if let Err(validation_error) = codec.validate() {
        eprintln!("⚠️  Validation warnings found");
        eprintln!("Warning: {}", validation_error);
        // Continue anyway for debug purposes
    } else {
        eprintln!("✅ Resources validated successfully");
    }

    // Replace \\n with \n in the resources
    for resource in &mut codec.resources {
        for entry in &mut resource.entries {
            entry.value = match &entry.value {
                Translation::Empty => Translation::Empty,
                Translation::Singular(v) => Translation::Singular(v.replace("\\n", "\n")),
                Translation::Plural(p) => Translation::Plural(Plural {
                    id: p.id.clone(),
                    forms: p
                        .forms
                        .clone()
                        .into_iter()
                        .map(|(k, v)| (k, v.replace("\\n", "\n")))
                        .collect(),
                }),
            };
        }
    }

    // Convert to JSON
    eprintln!("Converting to JSON...");
    let json = serde_json::to_string_pretty(&*codec.resources).unwrap_or_else(|e| {
        eprintln!("❌ Error serializing to JSON");
        eprintln!("Error serializing to JSON: {}", e);
        std::process::exit(1);
    });

    // Output to file or stdout
    let output_to_file = match output {
        Some(output_path) => {
            eprintln!("Writing output file...");
            if let Err(e) =
                File::create(&output_path).and_then(|mut f| f.write_all(json.as_bytes()))
            {
                eprintln!("❌ Error writing output file");
                eprintln!("Error writing to {}: {}", output_path, e);
                std::process::exit(1);
            }
            eprintln!("✅ Debug output written to: {}", output_path);
            true
        }
        None => {
            // Write marker + JSON to stdout and gracefully handle Broken Pipe
            let mut stdout = io::stdout().lock();
            if let Err(e) = stdout.write_all("✅ Debug output:\n".as_bytes()) {
                if e.kind() == io::ErrorKind::BrokenPipe {
                    std::process::exit(0);
                }
                eprintln!("Error writing to stdout: {}", e);
                std::process::exit(1);
            }
            if let Err(e) = stdout.write_all(json.as_bytes()) {
                if e.kind() == io::ErrorKind::BrokenPipe {
                    // Downstream closed the pipe (e.g., `| head`). Exit quietly.
                    std::process::exit(0);
                }
                eprintln!("Error writing to stdout: {}", e);
                std::process::exit(1);
            }
            if let Err(e) = stdout.flush() {
                if e.kind() == io::ErrorKind::BrokenPipe {
                    std::process::exit(0);
                }
                eprintln!("Error flushing stdout: {}", e);
                std::process::exit(1);
            }
            false
        }
    };

    // Show additional debug information using the new high-level methods
    if !output_to_file {
        eprintln!("\n=== Debug Summary ===");
        eprintln!(
            "Languages: {}",
            codec.languages().collect::<Vec<_>>().join(", ")
        );
        eprintln!("Total entries: {}", codec.all_keys().count());

        for lang in codec.languages() {
            let count = codec.entry_count(lang);
            eprintln!("  {}: {} entries", lang, count);
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
