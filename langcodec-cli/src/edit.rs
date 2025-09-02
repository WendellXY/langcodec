use crate::path_glob;
use crate::validation::{ValidationContext, validate_context};
use langcodec::{Codec, Resource, Translation, formats::FormatType};
use std::path::Path;

fn parse_status_opt(
    status: &Option<String>,
) -> Result<Option<langcodec::types::EntryStatus>, String> {
    if let Some(s) = status {
        let normalized = s.replace(['-', ' '], "_");
        normalized
            .parse::<langcodec::types::EntryStatus>()
            .map(Some)
            .map_err(|e| e.to_string())
    } else {
        Ok(None)
    }
}

fn infer_output_format_from_path(path: &str) -> Result<FormatType, String> {
    langcodec::infer_format_from_extension(path)
        .ok_or_else(|| format!("Cannot infer format from path: {}", path))
}

fn pick_single_resource<'a>(
    codec: &'a Codec,
    lang: &Option<String>,
) -> Result<&'a Resource, String> {
    if let Some(l) = lang {
        codec
            .get_by_language(l)
            .ok_or_else(|| format!("Language '{}' not found in input", l))
    } else if codec.resources.len() == 1 {
        Ok(&codec.resources[0])
    } else {
        Err("Multiple languages present; specify --lang".to_string())
    }
}

fn pick_single_resource_mut<'a>(
    codec: &'a mut Codec,
    lang: &Option<String>,
) -> Result<&'a mut Resource, String> {
    if let Some(l) = lang {
        codec
            .get_mut_by_language(l)
            .ok_or_else(|| format!("Language '{}' not found in input", l))
    } else if codec.resources.len() == 1 {
        Ok(&mut codec.resources[0])
    } else {
        Err("Multiple languages present; specify --lang".to_string())
    }
}

fn write_back(
    codec: &Codec,
    input_path: &str,
    output_path: &Option<String>,
    lang: &Option<String>,
) -> Result<(), String> {
    let input_owned = input_path.to_string();
    let out = output_path.as_ref().unwrap_or(&input_owned);
    let fmt = infer_output_format_from_path(out)?;

    match fmt {
        FormatType::Strings(_) | FormatType::AndroidStrings(_) => {
            // Single-language per file formats: write only one resource
            let res = pick_single_resource(codec, lang)?;
            langcodec::Codec::write_resource_to_file(res, out)
                .map_err(|e| format!("Error writing output: {}", e))
        }
        FormatType::Xcstrings | FormatType::CSV | FormatType::TSV => {
            // Multi-language formats: write all resources
            let resources = codec.resources.clone();
            langcodec::converter::convert_resources_to_format(resources, out, fmt)
                .map_err(|e| format!("Error writing output: {}", e))
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[derive(Debug, Clone)]
pub struct EditSetOptions {
    pub inputs: Vec<String>,
    pub lang: Option<String>,
    pub key: String,
    pub value: Option<String>,
    pub comment: Option<String>,
    pub status: Option<String>,
    pub output: Option<String>,
    pub dry_run: bool,
}

pub fn run_edit_set_command(opts: EditSetOptions) -> Result<(), String> {
    // Expand globs for inputs
    let expanded = path_glob::expand_input_globs(&opts.inputs)
        .map_err(|e| format!("Failed to expand input patterns: {}", e))?;
    if expanded.is_empty() {
        return Err("No input files matched the provided patterns".to_string());
    }

    if expanded.len() > 1 && opts.output.is_some() {
        return Err("--output cannot be used with multiple input files".to_string());
    }

    for input_path in expanded {
        // Validate per-file
        let mut vctx = ValidationContext::new().with_input_file(input_path.clone());
        if let Some(l) = &opts.lang {
            vctx = vctx.with_language_code(l.clone());
        }
        if let Some(o) = &opts.output {
            vctx = vctx.with_output_file(o.clone());
        }
        validate_context(&vctx)
            .map_err(|e| format!("Input validation failed for '{}': {}", input_path, e))?;

        apply_set_to_file(
            &input_path,
            &opts.lang,
            &opts.key,
            &opts.value,
            &opts.comment,
            &opts.status,
            opts.output.as_ref(),
            opts.dry_run,
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_set_to_file(
    input: &str,
    lang: &Option<String>,
    key: &str,
    value: &Option<String>,
    comment: &Option<String>,
    status: &Option<String>,
    output: Option<&String>,
    dry_run: bool,
) -> Result<(), String> {
    let mut codec = Codec::new();
    if let Err(e) = codec.read_file_by_extension(input, lang.clone()) {
        let ext = Path::new(input)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if matches!(ext, "json" | "yaml" | "yml" | "langcodec") {
            return Err(
                "Edit currently supports standard formats (.strings, .xml, .xcstrings, .csv, .tsv)"
                    .to_string(),
            );
        }
        return Err(format!("Failed to read input '{}': {}", input, e));
    }

    let is_remove = value.as_deref().map(|s| s.is_empty()).unwrap_or(true);
    let status_parsed = parse_status_opt(status)?;

    if is_remove {
        if let Some(l) = lang.as_deref() {
            if codec.has_entry(key, l) {
                if dry_run {
                    println!("DRY-RUN: Would remove '{}' from {} ({})", key, l, input);
                } else {
                    codec.remove_entry(key, l).map_err(|e| e.to_string())?;
                    println!("✅ Removed '{}' from {} ({})", key, l, input);
                }
            } else {
                println!(
                    "ℹ️  Key '{}' not found in {} ({}); nothing to remove",
                    key, l, input
                );
            }
        } else {
            let res = pick_single_resource_mut(&mut codec, lang)?;
            let before = res.entries.len();
            let will_remove = res.entries.iter().any(|e| e.id == key);
            if dry_run {
                if will_remove {
                    println!("DRY-RUN: Would remove '{}' ({})", key, input);
                } else {
                    println!(
                        "ℹ️  Key '{}' not present ({}); nothing to remove",
                        key, input
                    );
                }
            } else {
                res.entries.retain(|e| e.id != key);
                if res.entries.len() < before {
                    println!("✅ Removed '{}' ({})", key, input);
                } else {
                    println!(
                        "ℹ️  Key '{}' not present ({}); nothing to remove",
                        key, input
                    );
                }
            }
        }
    } else {
        let resolved_lang_owned: String;
        let lref: &str = if let Some(l) = lang.as_deref() {
            l
        } else if codec.resources.len() == 1 {
            resolved_lang_owned = codec.resources[0].metadata.language.clone();
            resolved_lang_owned.as_str()
        } else {
            return Err(format!(
                "--lang is required for set on multi-language files ({})",
                input
            ));
        };
        let val = value.clone().unwrap_or_default();
        let exists = codec.has_entry(key, lref);
        if exists {
            let old = codec
                .find_entry(key, lref)
                .map(|e| match &e.value {
                    Translation::Singular(s) => s.clone(),
                    Translation::Plural(p) => p.id.clone(),
                })
                .unwrap_or_default();
            if dry_run {
                println!(
                    "DRY-RUN: Would update '{}' in {}: '{}' -> '{}' ({})",
                    key, lref, old, val, input
                );
            } else {
                codec
                    .update_translation(
                        key,
                        lref,
                        Translation::Singular(val.clone()),
                        status_parsed.clone(),
                    )
                    .map_err(|e| e.to_string())?;
                if comment.is_some()
                    && let Some(entry) = codec.find_entry_mut(key, lref)
                {
                    entry.comment = comment.clone();
                }
                println!("✅ Updated '{}' in {} ({})", key, lref, input);
            }
        } else if dry_run {
            println!(
                "DRY-RUN: Would add '{}' to {} with value '{}' ({})",
                key, lref, val, input
            );
        } else {
            codec
                .add_entry(
                    key,
                    lref,
                    Translation::Singular(val.clone()),
                    comment.clone(),
                    status_parsed.clone(),
                )
                .map_err(|e| e.to_string())?;
            println!("✅ Added '{}' to {} ({})", key, lref, input);
        }
    }

    if !dry_run {
        write_back(&codec, input, &output.cloned(), lang)?;
        if let Some(out) = output {
            println!("📄 Wrote changes to {}", out);
        } else {
            println!("📄 Updated {} in place", input);
        }
    }

    Ok(())
}
