use crate::path_glob;
use crate::validation::{validate_file_path, validate_output_path};
use langcodec::{
    Codec, FormatType, KeyStyle, NormalizeOptions as EngineNormalizeOptions, normalize_codec,
};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct NormalizeCliOptions {
    pub inputs: Vec<String>,
    pub output: Option<String>,
    pub dry_run: bool,
    pub check: bool,
    pub no_placeholders: bool,
    pub key_style: String,
}

fn parse_key_style(input: &str) -> Result<KeyStyle, String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(KeyStyle::None),
        "snake" => Ok(KeyStyle::Snake),
        "kebab" => Ok(KeyStyle::Kebab),
        "camel" => Ok(KeyStyle::Camel),
        other => Err(format!(
            "Invalid --key-style '{}'. Expected one of: none, snake, kebab, camel",
            other
        )),
    }
}

fn infer_output_format_from_path(path: &str) -> Result<FormatType, String> {
    langcodec::infer_format_from_extension(path)
        .ok_or_else(|| format!("Cannot infer format from path: {}", path))
}

fn pick_single_resource(codec: &Codec) -> Result<&langcodec::Resource, String> {
    if codec.resources.len() == 1 {
        Ok(&codec.resources[0])
    } else {
        Err(
            "Multiple languages present; single-language output requires exactly one resource"
                .to_string(),
        )
    }
}

fn write_back(codec: &Codec, input_path: &str, output_path: &Option<String>) -> Result<(), String> {
    let input_owned = input_path.to_string();
    let out = output_path.as_ref().unwrap_or(&input_owned);
    let fmt = infer_output_format_from_path(out)?;

    match fmt {
        FormatType::Strings(_) | FormatType::AndroidStrings(_) => {
            let resource = pick_single_resource(codec)?;
            Codec::write_resource_to_file(resource, out)
                .map_err(|e| format!("Error writing output: {}", e))
        }
        FormatType::Xcstrings | FormatType::CSV | FormatType::TSV => {
            langcodec::converter::convert_resources_to_format(codec.resources.clone(), out, fmt)
                .map_err(|e| format!("Error writing output: {}", e))
        }
    }
}

fn has_distinct_output_path(input_path: &str, output_path: &Option<String>) -> bool {
    output_path
        .as_ref()
        .is_some_and(|output| Path::new(output) != Path::new(input_path))
}

pub fn run_normalize_command(opts: NormalizeCliOptions) -> Result<(), String> {
    let expanded = path_glob::expand_input_globs(&opts.inputs)
        .map_err(|e| format!("Failed to expand input patterns: {}", e))?;
    if expanded.is_empty() {
        return Err("No input files matched the provided patterns".to_string());
    }
    if expanded.len() > 1 {
        return Err("Normalize currently supports exactly one input file".to_string());
    }

    let input = &expanded[0];
    validate_file_path(input)?;

    let mut codec = Codec::new();
    codec
        .read_file_by_extension(input, None)
        .map_err(|e| format!("Failed to read input '{}': {}", input, e))?;

    let key_style = parse_key_style(&opts.key_style)?;
    let report = normalize_codec(
        &mut codec,
        &EngineNormalizeOptions {
            normalize_placeholders: !opts.no_placeholders,
            key_style,
        },
    )
    .map_err(|e| e.to_string())?;

    if opts.check {
        if report.changed {
            println!("would change: {}", input);
            return Err(format!("would change: {}", input));
        }

        println!("No changes needed: {}", input);
        return Ok(());
    }

    if opts.dry_run {
        if report.changed {
            println!("DRY-RUN: would change {}", input);
        } else {
            println!("No changes needed: {}", input);
        }
        return Ok(());
    }

    if !report.changed {
        if has_distinct_output_path(input, &opts.output) {
            if let Some(output) = &opts.output {
                validate_output_path(output)?;
            }
            write_back(&codec, input, &opts.output)?;
            println!("No changes needed: {}", input);
            println!("✅ Wrote output: {}", opts.output.as_deref().unwrap_or(input));
            return Ok(());
        }

        println!("No changes needed: {}", input);
        return Ok(());
    }

    if let Some(output) = &opts.output {
        validate_output_path(output)?;
    }

    write_back(&codec, input, &opts.output)?;
    println!("✅ Normalized: {}", opts.output.as_deref().unwrap_or(input));
    Ok(())
}
