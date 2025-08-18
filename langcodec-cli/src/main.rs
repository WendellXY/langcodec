mod debug;
mod formats;
mod merge;
mod transformers;
mod validation;
mod view;

use crate::debug::run_debug_command;
use crate::formats::parse_custom_format;
use crate::merge::{ConflictStrategy, run_merge_command};
use crate::transformers::custom_format_to_resource;
use crate::validation::{ValidationContext, validate_context, validate_custom_format_file};
use crate::view::print_view;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use langcodec::{Codec, convert_auto, formats::FormatType};
use std::fs::File;
use std::io::BufWriter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    commands: Commands,
}

/// Supported subcommands.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Convert localization files between formats.
    ///
    /// This command automatically detects input and output formats from file extensions.
    /// For JSON files, it will try multiple parsing strategies:
    /// - Standard Resource format (if supported by langcodec)
    /// - JSON key-value pairs (for custom JSON formats)
    Convert {
        /// The input file to process
        #[arg(short, long)]
        input: String,
        /// The output file to write the results to
        #[arg(short, long)]
        output: String,
        /// Optional input format hint (e.g., "json-language-map", "json-array-language-map", "yaml-language-map", "strings", "android")
        #[arg(long)]
        input_format: Option<String>,
        /// Optional output format hint (e.g., "xcstrings", "strings", "android")
        #[arg(long)]
        output_format: Option<String>,
        /// Language codes to exclude from output (e.g., "en", "fr"). Can be specified multiple times.
        #[arg(long, value_name = "LANG")]
        exclude_lang: Vec<String>,
        /// Language codes to include in output (e.g., "en", "fr"). Can be specified multiple times. If specified, only these languages will be included.
        #[arg(long, value_name = "LANG")]
        include_lang: Vec<String>,
    },

    /// View localization files.
    View {
        /// The input file to view
        #[arg(short, long)]
        input: String,

        /// Optional language code to filter entries by
        #[arg(short, long)]
        lang: Option<String>,

        /// Display full value without truncation (even in terminal)
        #[arg(long)]
        full: bool,
    },

    /// Merge multiple localization files of the same format into one output file.
    Merge {
        /// The input files to merge
        #[arg(short, long, num_args = 1..)]
        inputs: Vec<String>,
        /// The output file to write the merged results to
        #[arg(short, long)]
        output: String,
        /// Strategy for handling conflicts
        #[arg(short, long, default_value = "last")]
        strategy: ConflictStrategy,
        /// Language code to use for all input files (e.g., "en", "fr")
        #[arg(short, long)]
        lang: Option<String>,
    },

    /// Debug: Read a localization file and output as JSON.
    Debug {
        /// The input file to debug
        #[arg(short, long)]
        input: String,
        /// Language code to use (e.g., "en", "fr")
        #[arg(short, long)]
        lang: Option<String>,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() {
    let args = Args::parse();

    match args.commands {
        Commands::Convert {
            input,
            output,
            input_format,
            output_format,
            exclude_lang,
            include_lang,
        } => {
            // Create validation context
            let mut context = ValidationContext::new()
                .with_input_file(input.clone())
                .with_output_file(output.clone());

            if let Some(format) = &input_format {
                context = context.with_input_format(format.clone());
            }
            if let Some(format) = &output_format {
                context = context.with_output_format(format.clone());
            }

            // Validate all inputs
            if let Err(e) = validate_context(&context) {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }

            run_unified_convert_command(
                input,
                output,
                input_format,
                output_format,
                exclude_lang,
                include_lang,
            );
        }
        Commands::View { input, lang, full } => {
            // Create validation context
            let mut context = ValidationContext::new().with_input_file(input.clone());

            if let Some(lang_code) = &lang {
                context = context.with_language_code(lang_code.clone());
            }

            // Validate all inputs
            if let Err(e) = validate_context(&context) {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }

            // Read the input file using the traditional method
            let mut codec = Codec::new();

            // Try standard format first
            if let Ok(()) = codec.read_file_by_extension(&input, lang.clone()) {
                // Standard format succeeded
            } else if input.ends_with(".json")
                || input.ends_with(".yaml")
                || input.ends_with(".yml")
                || input.ends_with(".langcodec")
            {
                // Try custom format for JSON/YAML/langcodec files
                if let Err(e) = try_custom_format_view(&input, lang.clone(), &mut codec) {
                    eprintln!("Failed to read file: {}", e);
                    std::process::exit(1);
                }
            } else {
                eprintln!("Failed to read file: unsupported format");
                std::process::exit(1);
            }

            print_view(&codec, &lang, full);
        }
        Commands::Merge {
            inputs,
            output,
            strategy,
            lang,
        } => {
            // Create validation context
            let mut context = ValidationContext::new().with_output_file(output.clone());

            for input in &inputs {
                context = context.with_input_file(input.clone());
            }

            if let Some(lang_code) = &lang {
                context = context.with_language_code(lang_code.clone());
            }

            // Validate all inputs
            if let Err(e) = validate_context(&context) {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }

            run_merge_command(inputs, output, strategy, lang);
        }
        Commands::Debug {
            input,
            lang,
            output,
        } => {
            // Create validation context
            let mut context = ValidationContext::new().with_input_file(input.clone());

            if let Some(lang_code) = &lang {
                context = context.with_language_code(lang_code.clone());
            }
            if let Some(output_path) = &output {
                context = context.with_output_file(output_path.clone());
            }

            // Validate all inputs
            if let Err(e) = validate_context(&context) {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }

            run_debug_command(input, lang, output);
        }
    }
}

fn run_unified_convert_command(
    input: String,
    output: String,
    input_format: Option<String>,
    output_format: Option<String>,
    exclude_lang: Vec<String>,
    include_lang: Vec<String>,
) {
    // Create progress bar
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );
    progress_bar.set_message("Detecting input format...");

    // If the desired output is .langcodec, handle via resource serialization
    if output.ends_with(".langcodec") {
        progress_bar.set_message("Converting input to .langcodec (Resource JSON array)...");
        match read_resources_from_any_input(&input, input_format.as_ref()).and_then(|resources| {
            // Apply language filtering
            let filtered_resources = resources
                .into_iter()
                .filter(|resource| {
                    let lang = &resource.metadata.language;

                    // If include_lang is specified, only include those languages
                    if !include_lang.is_empty() && !include_lang.contains(lang) {
                        return false;
                    }

                    // If exclude_lang is specified, exclude those languages
                    if !exclude_lang.is_empty() && exclude_lang.contains(lang) {
                        return false;
                    }

                    true
                })
                .collect();

            write_resources_as_langcodec(&filtered_resources, &output)
        }) {
            Ok(()) => {
                progress_bar.finish_with_message(
                    "✅ Successfully converted to .langcodec (Resource JSON array)",
                );
                return;
            }
            Err(e) => {
                progress_bar.finish_with_message("❌ Conversion to .langcodec failed");
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Strategy 1: Try standard lib crate conversion first
    progress_bar.set_message("Trying standard format detection...");
    if let Ok(()) = convert_auto(&input, &output) {
        progress_bar
            .finish_with_message("✅ Successfully converted using standard format detection");
        return;
    }

    // Strategy 2: Try custom formats for JSON/YAML/langcodec files
    if input.ends_with(".json")
        || input.ends_with(".yaml")
        || input.ends_with(".yml")
        || input.ends_with(".langcodec")
    {
        // For JSON files without explicit format, try standard format detection first
        if input.ends_with(".json") && input_format.is_none() {
            progress_bar.set_message("Trying standard JSON format detection...");
            // Try to use the standard format detection which will show proper JSON parsing errors
            if let Err(e) = convert_auto(&input, &output) {
                progress_bar.set_message("Trying custom JSON format conversion...");
                // If standard detection fails, try custom formats
                if let Ok(()) = try_custom_format_conversion(&input, &output, &input_format) {
                    progress_bar
                        .finish_with_message("✅ Successfully converted using custom JSON format");
                    return;
                }
                // If both fail, show the standard error message
                progress_bar.finish_with_message("❌ Conversion failed");
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        } else {
            // For YAML and langcodec files, try custom formats directly
            progress_bar.set_message("Converting using custom format...");
            if let Err(e) = try_custom_format_conversion(&input, &output, &input_format) {
                progress_bar.finish_with_message("❌ Custom format conversion failed");
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            progress_bar.finish_with_message("✅ Successfully converted using custom format");
            return;
        }
    }

    // Strategy 3: If we have format hints, try with explicit formats
    if let (Some(input_fmt), Some(output_fmt)) = (input_format, output_format) {
        progress_bar.set_message("Converting with explicit format hints...");
        if let Err(e) = try_explicit_format_conversion(&input, &output, &input_fmt, &output_fmt) {
            progress_bar.finish_with_message("❌ Explicit format conversion failed");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        progress_bar.finish_with_message("✅ Successfully converted with explicit formats");
        return;
    }

    // If all strategies failed, provide helpful error message
    progress_bar.finish_with_message("❌ All conversion strategies failed");
    print_conversion_error(&input, &output);
    std::process::exit(1);
}

fn try_custom_format_conversion(
    input: &str,
    output: &str,
    input_format: &Option<String>,
) -> Result<(), String> {
    // Validate custom format file
    validate_custom_format_file(input)?;

    let custom_format = if let Some(format_str) = input_format {
        parse_custom_format(format_str)?
    } else {
        // Auto-detect format based on file content
        let file_content = std::fs::read_to_string(input)
            .map_err(|e| format!("Error reading file {}: {}", input, e))?;

        // Validate file content
        formats::validate_custom_format_content(input, &file_content)?
    };

    // Convert custom format to Resource
    let resources = custom_format_to_resource(input.to_string(), custom_format)?;

    // If output is .langcodec, serialize resources as JSON array
    if output.ends_with(".langcodec") {
        write_resources_as_langcodec(&resources, output)?;
        return Ok(());
    }

    // Get output format type
    let output_format_type = langcodec::infer_format_from_extension(output)
        .ok_or_else(|| format!("Cannot infer output format from extension: {}", output))?;

    // Convert to target format
    convert_resources_to_format(resources, output, output_format_type)
        .map_err(|e| format!("Error converting to output format: {}", e))?;

    Ok(())
}

fn print_conversion_error(input: &str, output: &str) {
    eprintln!("Error: Could not convert {} to {}", input, output);
    eprintln!();
    eprintln!("Tried the following strategies:");
    eprintln!("1. Standard format detection from file extensions");
    if input.ends_with(".json") {
        eprintln!("2. Custom JSON format conversion");
    }
    if input.ends_with(".yaml") || input.ends_with(".yml") {
        eprintln!("2. Custom YAML format conversion");
    }
    if input.ends_with(".langcodec") {
        eprintln!("2. Custom langcodec Resource array format conversion");
    }
    eprintln!();
    eprintln!("Supported input formats:");
    eprintln!("- .strings (Apple strings files)");
    eprintln!("- .xml (Android strings files)");
    eprintln!("- .xcstrings (Apple xcstrings files)");
    eprintln!("- .csv (CSV files)");
    eprintln!("- .tsv (TSV files)");
    eprintln!("- .langcodec (Resource JSON array)");
    eprintln!("- .json (JSON key-value pairs or Resource format)");
    eprintln!("- .yaml/.yml (YAML language map format)");
    eprintln!("- .langcodec (JSON array of langcodec::Resource objects)");
    eprintln!();
    eprintln!("Supported output formats:");
    eprintln!("- .strings (Apple strings files)");
    eprintln!("- .xml (Android strings files)");
    eprintln!("- .xcstrings (Apple xcstrings files)");
    eprintln!("- .csv (CSV files)");
    eprintln!("- .tsv (TSV files)");
    eprintln!();
    eprintln!(
        "For JSON files, the command will try both standard Resource format and key-value pairs."
    );
    eprintln!("For YAML files, the command will try YAML language map format.");
    eprintln!(
        "Custom formats: {}",
        formats::get_supported_custom_formats()
    );
}

/// Convert a Vec<Resource> to a specific output format using the lib crate
fn convert_resources_to_format(
    resources: Vec<langcodec::Resource>,
    output: &str,
    output_format: FormatType,
) -> Result<(), langcodec::Error> {
    langcodec::Codec::convert_resources_to_format(resources, output, output_format)
}

/// Try explicit format conversion with specified input and output formats
fn try_explicit_format_conversion(
    input: &str,
    output: &str,
    input_format: &str,
    output_format: &str,
) -> Result<(), String> {
    // Validate input file exists
    validation::validate_file_path(input)?;

    // Validate output path
    validation::validate_output_path(output)?;

    // Parse input format
    let input_format_type = match input_format.to_lowercase().as_str() {
        "strings" => langcodec::formats::FormatType::Strings(None),
        "android" | "androidstrings" => langcodec::formats::FormatType::AndroidStrings(None),
        "xcstrings" => langcodec::formats::FormatType::Xcstrings,
        "csv" => langcodec::formats::FormatType::CSV,
        "tsv" => langcodec::formats::FormatType::TSV,
        _ => return Err(format!("Unsupported input format: {}", input_format)),
    };

    // Handle .langcodec output specially by reading resources then serializing
    if output_format.to_lowercase().as_str() == "langcodec" || output.ends_with(".langcodec") {
        // Read resources using explicit input format
        let mut codec = Codec::new();
        codec
            .read_file_by_type(input, input_format_type)
            .map_err(|e| format!("Failed to read input with explicit format: {}", e))?;
        write_resources_as_langcodec(&codec.resources, output)
    } else {
        // Parse output format
        let output_format_type = match output_format.to_lowercase().as_str() {
            "strings" => langcodec::formats::FormatType::Strings(None),
            "android" | "androidstrings" => langcodec::formats::FormatType::AndroidStrings(None),
            "xcstrings" => langcodec::formats::FormatType::Xcstrings,
            "csv" => langcodec::formats::FormatType::CSV,
            "tsv" => langcodec::formats::FormatType::TSV,
            _ => return Err(format!("Unsupported output format: {}", output_format)),
        };

        // Use the lib crate's convert function
        langcodec::convert(input, input_format_type, output, output_format_type)
            .map_err(|e| format!("Conversion error: {}", e))
    }
}

/// Try to read a custom format file and add it to the codec for view
fn try_custom_format_view(
    input: &str,
    _lang: Option<String>,
    codec: &mut Codec,
) -> Result<(), String> {
    // Validate custom format file
    validation::validate_custom_format_file(input)?;

    // Auto-detect format based on file content
    let file_content = std::fs::read_to_string(input)
        .map_err(|e| format!("Error reading file {}: {}", input, e))?;

    // Validate file content
    let custom_format = formats::validate_custom_format_content(input, &file_content)?;

    // Convert custom format to Resource
    let resources = custom_format_to_resource(input.to_string(), custom_format)?;

    // Add resources to codec
    for resource in resources {
        codec.add_resource(resource);
    }

    Ok(())
}

/// Serialize resources as a .langcodec (Resource JSON array) file
fn write_resources_as_langcodec(
    resources: &Vec<langcodec::Resource>,
    output: &str,
) -> Result<(), String> {
    let file = File::create(output).map_err(|e| format!("Failed to create {}: {}", output, e))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, resources)
        .map_err(|e| format!("Failed to write .langcodec JSON: {}", e))
}

/// Read resources from any supported input (standard or custom formats)
fn read_resources_from_any_input(
    input: &str,
    input_format_hint: Option<&String>,
) -> Result<Vec<langcodec::Resource>, String> {
    // First: if explicit input format is provided and is a standard format, use it
    if let Some(fmt) = input_format_hint {
        let fmt_lower = fmt.to_lowercase();
        let maybe_std = match fmt_lower.as_str() {
            "strings" => Some(langcodec::formats::FormatType::Strings(None)),
            "android" | "androidstrings" => {
                Some(langcodec::formats::FormatType::AndroidStrings(None))
            }
            "xcstrings" => Some(langcodec::formats::FormatType::Xcstrings),
            "csv" => Some(langcodec::formats::FormatType::CSV),
            "tsv" => Some(langcodec::formats::FormatType::TSV),
            _ => None,
        };
        if let Some(std_fmt) = maybe_std {
            // Try using the builder pattern which might handle language inference better
            match Codec::builder().add_file(input) {
                Ok(codec) => return Ok(codec.build().resources),
                Err(e) => {
                    // Fall back to manual approach with language inference
                    let lang_from_filename = input
                        .split('/')
                        .next_back()
                        .and_then(|name| name.split('.').next())
                        .and_then(|name| {
                            if name.len() == 2 && name.chars().all(|c| c.is_ascii_lowercase()) {
                                Some(name.to_string())
                            } else if name.contains('_') {
                                // Handle cases like "sample_en"
                                name.split('_').next_back().map(|s| s.to_string())
                            } else {
                                None
                            }
                        });

                    if let Some(lang) = lang_from_filename {
                        let lang_clone = lang.clone();
                        let mut codec = Codec::new();
                        codec
                            .read_file_by_type(input, std_fmt.with_language(Some(lang)))
                            .map_err(|e2| {
                                format!(
                                    "Failed to read input with language '{}': {}",
                                    lang_clone, e2
                                )
                            })?;
                        return Ok(codec.resources);
                    } else {
                        return Err(format!("Failed to read input: {}", e));
                    }
                }
            }
        }
    }

    // Second: try reading as a standard file by extension using builder pattern
    match Codec::builder().add_file(input) {
        Ok(codec) => return Ok(codec.build().resources),
        Err(e) => {
            // Try manual language inference and format detection
            let lang_from_filename = input
                .split('/')
                .next_back()
                .and_then(|name| name.split('.').next())
                .and_then(|name| {
                    if name.len() == 2 && name.chars().all(|c| c.is_ascii_lowercase()) {
                        Some(name.to_string())
                    } else if name.contains('_') {
                        // Handle cases like "sample_en"
                        name.split('_').next_back().map(|s| s.to_string())
                    } else {
                        None
                    }
                });

            if let Some(lang) = lang_from_filename {
                // Try with explicit format and language
                let format_type = if input.ends_with(".strings") {
                    langcodec::formats::FormatType::Strings(Some(lang.clone()))
                } else if input.ends_with(".xml") {
                    langcodec::formats::FormatType::AndroidStrings(Some(lang.clone()))
                } else if input.ends_with(".xcstrings") {
                    langcodec::formats::FormatType::Xcstrings
                } else if input.ends_with(".csv") {
                    langcodec::formats::FormatType::CSV
                } else if input.ends_with(".tsv") {
                    langcodec::formats::FormatType::TSV
                } else {
                    return Err(format!("Unsupported file extension for input: {}", input));
                };

                let mut codec = Codec::new();
                codec.read_file_by_type(input, format_type).map_err(|e2| {
                    format!("Failed to read input with language '{}': {}", lang, e2)
                })?;
                return Ok(codec.resources);
            } else {
                eprintln!("Standard format detection failed: {}", e);
            }
        }
    }

    // Third: try custom formats (json, yaml, langcodec)
    if input.ends_with(".json")
        || input.ends_with(".yaml")
        || input.ends_with(".yml")
        || input.ends_with(".langcodec")
    {
        // Validate custom format file
        validate_custom_format_file(input)?;

        // Auto-detect format based on file content
        let file_content = std::fs::read_to_string(input)
            .map_err(|e| format!("Error reading file {}: {}", input, e))?;

        // Validate file content
        let custom_format = formats::validate_custom_format_content(input, &file_content)?;

        // Convert custom format to Resource
        let resources = custom_format_to_resource(input.to_string(), custom_format)?;
        return Ok(resources);
    }

    Err("Unsupported input format or file extension".to_string())
}
