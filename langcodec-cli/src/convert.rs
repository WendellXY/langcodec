use crate::formats::{self, parse_custom_format};
use crate::transformers::custom_format_to_resource;
use crate::validation::{self, validate_custom_format_file};

use langcodec::{Codec, convert_auto, formats::FormatType};
use std::fs::File;
use std::io::BufWriter;

#[derive(Debug, Clone)]
pub struct ConvertOptions {
    pub input_format: Option<String>,
    pub output_format: Option<String>,
    pub source_language: Option<String>,
    pub version: Option<String>,
    pub exclude_lang: Vec<String>,
    pub include_lang: Vec<String>,
}

pub fn run_unified_convert_command(
    input: String,
    output: String,
    options: ConvertOptions,
    strict: bool,
) {
    // Special handling: when targeting xcstrings, ensure required metadata exists.
    // If source_language/version are missing, default to en/1.0 respectively.
    let wants_xcstrings = output.ends_with(".xcstrings")
        || options
            .output_format
            .as_deref()
            .is_some_and(|s| s.eq_ignore_ascii_case("xcstrings"));
    if wants_xcstrings {
        println!("Converting to xcstrings with default sourceLanguage if missing...");
        match read_resources_from_any_input(&input, options.input_format.as_ref(), strict).and_then(
            |mut resources| {
                // Determine source_language priority: explicit flag > metadata > default
                let source_language = options
                    .source_language
                    .as_ref()
                    .and_then(|s| {
                        let trimmed = s.trim();
                        if !trimmed.is_empty() {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        resources.first().and_then(|r| {
                            r.metadata
                                .custom
                                .get("source_language")
                                .cloned()
                                .filter(|s| !s.trim().is_empty())
                        })
                    })
                    .unwrap_or_else(|| "en".to_string());
                // Determine version: keep existing if present; otherwise default to "1.0"
                let version = options
                    .version
                    .clone()
                    .or_else(|| {
                        resources
                            .first()
                            .and_then(|r| r.metadata.custom.get("version").cloned())
                    })
                    .unwrap_or_else(|| "1.0".to_string());

                // Apply to all resources so the writer has consistent metadata
                for r in &mut resources {
                    r.metadata
                        .custom
                        .insert("source_language".to_string(), source_language.clone());
                    r.metadata
                        .custom
                        .insert("version".to_string(), version.clone());
                }

                convert_resources_to_format(resources, &output, FormatType::Xcstrings)
                    .map_err(|e| format!("Error converting to xcstrings: {}", e))
            },
        ) {
            Ok(()) => {
                println!("✅ Successfully converted to xcstrings");
                return;
            }
            Err(e) => {
                println!("❌ Conversion to xcstrings failed");
                // Preserve legacy expectation for invalid JSON: surface an inference hint
                if input.ends_with(".json") {
                    eprintln!("Cannot infer input format");
                }
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    // If the desired output is .langcodec, handle via resource serialization
    if output.ends_with(".langcodec") {
        let filter_msg = if !options.include_lang.is_empty() || !options.exclude_lang.is_empty() {
            let mut parts = Vec::new();
            if !options.include_lang.is_empty() {
                parts.push(format!("including: {}", options.include_lang.join(", ")));
            }
            if !options.exclude_lang.is_empty() {
                parts.push(format!("excluding: {}", options.exclude_lang.join(", ")));
            }
            format!(" with language filtering ({})", parts.join(", "))
        } else {
            String::new()
        };

        println!(
            "Converting input to .langcodec (Resource JSON array){}...",
            filter_msg
        );
        match read_resources_from_any_input(&input, options.input_format.as_ref(), strict).and_then(
            |resources| {
                // Apply language filtering
                let filtered_resources = resources
                    .into_iter()
                    .filter(|resource| {
                        let lang = &resource.metadata.language;

                        // If include_lang is specified, only include those languages
                        if !options.include_lang.is_empty() && !options.include_lang.contains(lang)
                        {
                            return false;
                        }

                        // If exclude_lang is specified, exclude those languages
                        if !options.exclude_lang.is_empty() && options.exclude_lang.contains(lang) {
                            return false;
                        }

                        true
                    })
                    .collect();

                write_resources_as_langcodec(&filtered_resources, &output)
            },
        ) {
            Ok(()) => {
                let filter_msg =
                    if !options.include_lang.is_empty() || !options.exclude_lang.is_empty() {
                        let mut parts = Vec::new();
                        if !options.include_lang.is_empty() {
                            parts.push(format!("including: {}", options.include_lang.join(", ")));
                        }
                        if !options.exclude_lang.is_empty() {
                            parts.push(format!("excluding: {}", options.exclude_lang.join(", ")));
                        }
                        format!(" with language filtering ({})", parts.join(", "))
                    } else {
                        String::new()
                    };

                println!(
                    "✅ Successfully converted to .langcodec (Resource JSON array){}",
                    filter_msg
                );
                return;
            }
            Err(e) => {
                let filter_msg =
                    if !options.include_lang.is_empty() || !options.exclude_lang.is_empty() {
                        let mut parts = Vec::new();
                        if !options.include_lang.is_empty() {
                            parts.push(format!("including: {}", options.include_lang.join(", ")));
                        }
                        if !options.exclude_lang.is_empty() {
                            parts.push(format!("excluding: {}", options.exclude_lang.join(", ")));
                        }
                        format!(" with language filtering ({})", parts.join(", "))
                    } else {
                        String::new()
                    };

                println!("❌ Conversion to .langcodec failed{}", filter_msg);
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    if strict {
        if let (Some(input_fmt), Some(output_fmt)) = (
            options.input_format.as_deref(),
            options.output_format.as_deref(),
        ) {
            println!("Strict mode: converting with explicit format hints only...");
            if let Err(e) = try_explicit_format_conversion(&input, &output, input_fmt, output_fmt) {
                println!("❌ Strict conversion failed");
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            println!("✅ Successfully converted in strict mode");
            return;
        }

        if input.ends_with(".json")
            || input.ends_with(".yaml")
            || input.ends_with(".yml")
            || input.ends_with(".langcodec")
        {
            println!("Strict mode: converting custom format without fallback...");
            if let Err(e) = try_custom_format_conversion(&input, &output, &options.input_format) {
                println!("❌ Strict conversion failed");
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            println!("✅ Successfully converted in strict mode");
            return;
        }

        println!("Strict mode: converting using extension-based standard formats only...");
        if let Err(e) = convert_auto(&input, &output) {
            println!("❌ Strict conversion failed");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        println!("✅ Successfully converted in strict mode");
        return;
    }

    // Strategy 1: Try standard lib crate conversion first
    println!("Trying standard format detection from file extensions...");
    if let Ok(()) = convert_auto(&input, &output) {
        println!("✅ Successfully converted using standard format detection");
        return;
    }

    // Strategy 2: Try custom formats for JSON/YAML/langcodec files
    if input.ends_with(".json")
        || input.ends_with(".yaml")
        || input.ends_with(".yml")
        || input.ends_with(".langcodec")
    {
        // For JSON files without explicit format, try standard format detection first
        if input.ends_with(".json") && options.input_format.is_none() {
            println!("Trying standard JSON format detection...");
            // Try to use the standard format detection which will show proper JSON parsing errors
            if let Err(e) = convert_auto(&input, &output) {
                println!("Trying custom JSON format conversion...");
                // If standard detection fails, try custom formats
                if let Ok(()) = try_custom_format_conversion(&input, &output, &options.input_format)
                {
                    println!("✅ Successfully converted using custom JSON format");
                    return;
                }
                // If both fail, show the standard error message
                println!("❌ Conversion failed");
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        } else {
            // For YAML and langcodec files, try custom formats directly
            println!("Converting using custom format...");
            if let Err(e) = try_custom_format_conversion(&input, &output, &options.input_format) {
                println!("❌ Custom format conversion failed");
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            println!("✅ Successfully converted using custom format");
            return;
        }
    }

    // Strategy 3: If we have format hints, try with explicit formats
    if let (Some(input_fmt), Some(output_fmt)) =
        (options.input_format.clone(), options.output_format.clone())
    {
        println!("Converting with explicit format hints...");
        if let Err(e) = try_explicit_format_conversion(&input, &output, &input_fmt, &output_fmt) {
            println!("❌ Explicit format conversion failed");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        println!("✅ Successfully converted with explicit formats");
        return;
    }

    // If all strategies failed, provide helpful error message
    println!("❌ All conversion strategies failed");
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
    eprintln!("Error: Could not convert '{}' to '{}'", input, output);
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
    eprintln!();
    eprintln!("Supported output formats:");
    eprintln!("- .strings (Apple strings files)");
    eprintln!("- .xml (Android strings files)");
    eprintln!("- .xcstrings (Apple xcstrings files)");
    eprintln!("- .csv (CSV files)");
    eprintln!("- .tsv (TSV files)");
    eprintln!("- .langcodec (Resource JSON array)");
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
    langcodec::converter::convert_resources_to_format(resources, output, output_format)
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
        _ => {
            return Err(format!(
                "Unsupported input format: '{}'. Supported formats: strings, android, xcstrings, csv, tsv",
                input_format
            ));
        }
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
            _ => {
                return Err(format!(
                    "Unsupported output format: '{}'. Supported formats: strings, android, xcstrings, csv, tsv",
                    output_format
                ));
            }
        };

        // Use the lib crate's convert function
        langcodec::convert(input, input_format_type, output, output_format_type)
            .map_err(|e| format!("Conversion error: {}", e))
    }
}

/// Try to read a custom format file and add it to the codec for view
pub fn try_custom_format_view(
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
pub fn read_resources_from_any_input(
    input: &str,
    input_format_hint: Option<&String>,
    strict: bool,
) -> Result<Vec<langcodec::Resource>, String> {
    if strict {
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
                let mut codec = Codec::new();
                codec
                    .read_file_by_type(input, std_fmt)
                    .map_err(|e| format!("Failed to read input with explicit format: {}", e))?;
                return Ok(codec.resources);
            }

            let custom_format = parse_custom_format(fmt)?;
            let resources = custom_format_to_resource(input.to_string(), custom_format)?;
            return Ok(resources);
        }

        if input.ends_with(".strings")
            || input.ends_with(".xml")
            || input.ends_with(".xcstrings")
            || input.ends_with(".csv")
            || input.ends_with(".tsv")
        {
            let mut codec = Codec::new();
            codec
                .read_file_by_extension(input, None)
                .map_err(|e| format!("Failed to read input: {}", e))?;
            return Ok(codec.resources);
        }

        if input.ends_with(".json")
            || input.ends_with(".yaml")
            || input.ends_with(".yml")
            || input.ends_with(".langcodec")
        {
            validate_custom_format_file(input)?;
            let file_content = std::fs::read_to_string(input)
                .map_err(|e| format!("Error reading file {}: {}", input, e))?;
            let custom_format = formats::validate_custom_format_content(input, &file_content)?;
            let resources = custom_format_to_resource(input.to_string(), custom_format)?;
            return Ok(resources);
        }

        return Err(format!(
            "Unsupported input format or file extension: '{}'. Supported formats: .strings, .xml, .xcstrings, .csv, .tsv, .json, .yaml, .yml, .langcodec",
            input
        ));
    }

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
                let err_prefix = format!("Failed to read input with language '{}': ", lang);

                let format_type = if input.ends_with(".strings") {
                    langcodec::formats::FormatType::Strings(Some(lang))
                } else if input.ends_with(".xml") {
                    langcodec::formats::FormatType::AndroidStrings(Some(lang))
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
                codec
                    .read_file_by_type(input, format_type)
                    .map_err(|e2| format!("{err_prefix}{e2}"))?;
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

    Err(format!(
        "Unsupported input format or file extension: '{}'. Supported formats: .strings, .xml, .xcstrings, .csv, .tsv, .json, .yaml, .yml, .langcodec",
        input
    ))
}
