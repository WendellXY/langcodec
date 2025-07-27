mod debug;
mod formats;
mod merge;
mod transformers;
mod view;

use crate::debug::run_debug_command;
use crate::formats::parse_custom_format;
use crate::merge::{ConflictStrategy, run_merge_command};
use crate::transformers::custom_format_to_resource;
use crate::view::print_view;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use langcodec::{Codec, convert_auto, formats::FormatType, traits::Parser as CodecParser};

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
        } => {
            run_unified_convert_command(input, output, input_format, output_format);
        }
        Commands::View { input, lang, full } => {
            // Read the input file and print all the entries
            let mut codec = Codec::new();
            if let Err(e) = codec.read_file_by_extension(input, lang.clone()) {
                eprintln!("Failed to read file: {}", e);
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
            run_merge_command(inputs, output, strategy, lang);
        }
        Commands::Debug {
            input,
            lang,
            output,
        } => {
            run_debug_command(input, lang, output);
        }
    }
}

fn run_unified_convert_command(
    input: String,
    output: String,
    input_format: Option<String>,
    output_format: Option<String>,
) {
    // Create progress bar
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );
    progress_bar.set_message("Detecting input format...");

    // Strategy 1: Try standard lib crate conversion first
    progress_bar.set_message("Trying standard format detection...");
    if let Ok(()) = convert_auto(&input, &output) {
        progress_bar
            .finish_with_message("✅ Successfully converted using standard format detection");
        return;
    }

    // Strategy 2: Try custom formats for JSON files or when format is specified
    if input.ends_with(".json")
        || input.ends_with(".yaml")
        || input.ends_with(".yml")
        || input_format.is_some()
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
            // For YAML files or when format is specified, try custom formats directly
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
    let custom_format = if let Some(format_str) = input_format {
        parse_custom_format(format_str)?
    } else {
        // Auto-detect format based on file content
        let file_content = std::fs::read_to_string(input)
            .map_err(|e| format!("Error reading file {}: {}", input, e))?;

        if let Some(detected_format) = formats::detect_custom_format(input, &file_content) {
            detected_format
        } else {
            return Err(
                "Could not auto-detect custom format. Please specify --input-format.".to_string(),
            );
        }
    };

    // Convert custom format to Resource
    let resources = custom_format_to_resource(input.to_string(), custom_format)?;

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
    eprintln!();
    eprintln!("Supported input formats:");
    eprintln!("- .strings (Apple strings files)");
    eprintln!("- .xml (Android strings files)");
    eprintln!("- .xcstrings (Apple xcstrings files)");
    eprintln!("- .csv (CSV files)");
    eprintln!("- .json (JSON key-value pairs or Resource format)");
    eprintln!("- .yaml/.yml (YAML language map format)");
    eprintln!();
    eprintln!("Supported output formats:");
    eprintln!("- .strings (Apple strings files)");
    eprintln!("- .xml (Android strings files)");
    eprintln!("- .xcstrings (Apple xcstrings files)");
    eprintln!("- .csv (CSV files)");
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
    use langcodec::formats::{AndroidStringsFormat, CSVRecord, StringsFormat, XcstringsFormat};
    use std::path::Path;

    match output_format {
        FormatType::AndroidStrings(_) => {
            if let Some(resource) = resources.first() {
                AndroidStringsFormat::from(resource.clone()).write_to(Path::new(output))
            } else {
                Err(langcodec::Error::InvalidResource(
                    "No resources to convert".to_string(),
                ))
            }
        }
        FormatType::Strings(_) => {
            if let Some(resource) = resources.first() {
                StringsFormat::try_from(resource.clone())?.write_to(Path::new(output))
            } else {
                Err(langcodec::Error::InvalidResource(
                    "No resources to convert".to_string(),
                ))
            }
        }
        FormatType::Xcstrings => XcstringsFormat::try_from(resources)?.write_to(Path::new(output)),
        FormatType::CSV(_) => {
            if let Some(resource) = resources.first() {
                Vec::<CSVRecord>::try_from(resource.clone())?.write_to(Path::new(output))
            } else {
                Err(langcodec::Error::InvalidResource(
                    "No resources to convert".to_string(),
                ))
            }
        }
    }
}

/// Try explicit format conversion with specified input and output formats
fn try_explicit_format_conversion(
    input: &str,
    output: &str,
    input_format: &str,
    output_format: &str,
) -> Result<(), String> {
    // Parse input format
    let input_format_type = match input_format.to_lowercase().as_str() {
        "strings" => langcodec::formats::FormatType::Strings(None),
        "android" | "androidstrings" => langcodec::formats::FormatType::AndroidStrings(None),
        "xcstrings" => langcodec::formats::FormatType::Xcstrings,
        "csv" => langcodec::formats::FormatType::CSV(None),
        _ => return Err(format!("Unsupported input format: {}", input_format)),
    };

    // Parse output format
    let output_format_type = match output_format.to_lowercase().as_str() {
        "strings" => langcodec::formats::FormatType::Strings(None),
        "android" | "androidstrings" => langcodec::formats::FormatType::AndroidStrings(None),
        "xcstrings" => langcodec::formats::FormatType::Xcstrings,
        "csv" => langcodec::formats::FormatType::CSV(None),
        _ => return Err(format!("Unsupported output format: {}", output_format)),
    };

    // Use the lib crate's convert function
    langcodec::convert(input, input_format_type, output, output_format_type)
        .map_err(|e| format!("Conversion error: {}", e))
}
