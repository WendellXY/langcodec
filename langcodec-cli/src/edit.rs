use crate::validation::{validate_context, ValidationContext};
use langcodec::{formats::FormatType, Codec, Resource, Translation};
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
    pub input: String,
    pub lang: Option<String>,
    pub key: String,
    pub value: Option<String>,
    pub comment: Option<String>,
    pub status: Option<String>,
    pub output: Option<String>,
    pub dry_run: bool,
}

pub fn run_edit_set_command(opts: EditSetOptions) -> Result<(), String> {
    // Validate basic context
    let mut vctx = ValidationContext::new().with_input_file(opts.input.clone());
    if let Some(l) = &opts.lang {
        vctx = vctx.with_language_code(l.clone());
    }
    if let Some(o) = &opts.output {
        vctx = vctx.with_output_file(o.clone());
    }
    validate_context(&vctx).map_err(|e| e.to_string())?;

    // Load input
    let mut codec = Codec::new();
    if let Err(e) = codec.read_file_by_extension(&opts.input, opts.lang.clone()) {
        // Graceful hint for unsupported or custom formats
        let ext = Path::new(&opts.input)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if matches!(ext, "json" | "yaml" | "yml" | "langcodec") {
            return Err(
                "Edit currently supports standard formats (.strings, .xml, .xcstrings, .csv, .tsv)"
                    .to_string(),
            );
        }
        return Err(format!("Failed to read input: {}", e));
    }

    // Determine operation: remove if value is None or Some("")
    let is_remove = opts
        .value
        .as_deref()
        .map(|s| s.is_empty())
        .unwrap_or(true);
    let status_parsed = parse_status_opt(&opts.status)?;

    if is_remove {
        // Remove entry if present; treat missing as a no-op
        if let Some(ref l) = opts.lang {
            if codec.has_entry(&opts.key, l) {
                if opts.dry_run {
                    println!("DRY-RUN: Would remove '{}' from {}", opts.key, l);
                } else {
                    codec
                        .remove_entry(&opts.key, l)
                        .map_err(|e| e.to_string())?;
                    println!("✅ Removed '{}' from {}", opts.key, l);
                }
            } else {
                println!(
                    "ℹ️  Key '{}' not found in {}; nothing to remove",
                    opts.key, l
                );
            }
        } else {
            // No language specified: allow removal only if single resource present
            let res = pick_single_resource_mut(&mut codec, &opts.lang)?;
            let before = res.entries.len();
            let will_remove = res.entries.iter().any(|e| e.id == opts.key);
            if opts.dry_run {
                if will_remove {
                    println!("DRY-RUN: Would remove '{}'", opts.key);
                } else {
                    println!(
                        "ℹ️  Key '{}' not present; nothing to remove",
                        opts.key
                    );
                }
            } else {
                res.entries.retain(|e| e.id != opts.key);
                if res.entries.len() < before {
                    println!("✅ Removed '{}'", opts.key);
                } else {
                    println!(
                        "ℹ️  Key '{}' not present; nothing to remove",
                        opts.key
                    );
                }
            }
        }
    } else {
        // Add or update
        let resolved_lang_owned: String;
        let lref: &str = if let Some(l) = opts.lang.as_deref() {
            l
        } else if codec.resources.len() == 1 {
            resolved_lang_owned = codec.resources[0].metadata.language.clone();
            resolved_lang_owned.as_str()
        } else {
            return Err("--lang is required for set on multi-language files".to_string());
        };
        let val = opts.value.clone().unwrap_or_default();
        let exists = codec.has_entry(&opts.key, lref);
        if exists {
            let old = codec
                .find_entry(&opts.key, lref)
                .map(|e| match &e.value {
                    Translation::Singular(s) => s.clone(),
                    Translation::Plural(p) => p.id.clone(),
                })
                .unwrap_or_default();
            if opts.dry_run {
                println!(
                    "DRY-RUN: Would update '{}' in {}: '{}' -> '{}'",
                    opts.key, lref, old, val
                );
            } else {
                codec
                    .update_translation(
                        &opts.key,
                        lref,
                        Translation::Singular(val.clone()),
                        status_parsed.clone(),
                    )
                    .map_err(|e| e.to_string())?;
                if opts.comment.is_some()
                    && let Some(entry) = codec.find_entry_mut(&opts.key, lref)
                {
                    entry.comment = opts.comment.clone();
                }
                println!("✅ Updated '{}' in {}", opts.key, lref);
            }
        } else if opts.dry_run {
            println!(
                "DRY-RUN: Would add '{}' to {} with value '{}'",
                opts.key, lref, val
            );
        } else {
            codec
                .add_entry(
                    &opts.key,
                    lref,
                    Translation::Singular(val.clone()),
                    opts.comment.clone(),
                    status_parsed.clone(),
                )
                .map_err(|e| e.to_string())?;
            println!("✅ Added '{}' to {}", opts.key, lref);
        }
    }

    // Write back unless dry-run
    if !opts.dry_run {
        write_back(&codec, &opts.input, &opts.output, &opts.lang)?;
        if let Some(out) = &opts.output {
            println!("📄 Wrote changes to {}", out);
        } else {
            println!("📄 Updated {} in place", opts.input);
        }
    }

    Ok(())
}
