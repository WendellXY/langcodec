use crate::validation::{validate_context, ValidationContext};
use langcodec::{formats::FormatType, Codec, Resource, Translation};
use std::path::Path;

fn parse_status_opt(status: &Option<String>) -> Result<Option<langcodec::types::EntryStatus>, String> {
    if let Some(s) = status {
        let normalized = s.replace('-', "_").replace(' ', "_");
        normalized
            .parse::<langcodec::types::EntryStatus>()
            .map(Some)
            .map_err(|e| format!("{}", e))
    } else {
        Ok(None)
    }
}

fn infer_output_format_from_path(path: &str) -> Result<FormatType, String> {
    langcodec::infer_format_from_extension(path)
        .ok_or_else(|| format!("Cannot infer format from path: {}", path))
}

fn pick_single_resource<'a>(codec: &'a Codec, lang: &Option<String>) -> Result<&'a Resource, String> {
    if let Some(l) = lang {
        codec
            .get_by_language(l)
            .ok_or_else(|| format!("Language '{}' not found in input", l))
    } else {
        if codec.resources.len() == 1 {
            Ok(&codec.resources[0])
        } else {
            Err("Multiple languages present; specify --lang".to_string())
        }
    }
}

fn pick_single_resource_mut<'a>(codec: &'a mut Codec, lang: &Option<String>) -> Result<&'a mut Resource, String> {
    if let Some(l) = lang {
        codec
            .get_mut_by_language(l)
            .ok_or_else(|| format!("Language '{}' not found in input", l))
    } else {
        if codec.resources.len() == 1 {
            Ok(&mut codec.resources[0])
        } else {
            Err("Multiple languages present; specify --lang".to_string())
        }
    }
}

fn write_back(codec: &Codec, input_path: &str, output_path: &Option<String>, lang: &Option<String>) -> Result<(), String> {
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

pub fn run_edit_set_command(
    input: String,
    lang: Option<String>,
    key: String,
    value: Option<String>,
    comment: Option<String>,
    status: Option<String>,
    output: Option<String>,
) -> Result<(), String> {
    // Validate basic context
    let mut vctx = ValidationContext::new().with_input_file(input.clone());
    if let Some(l) = &lang {
        vctx = vctx.with_language_code(l.clone());
    }
    if let Some(o) = &output {
        vctx = vctx.with_output_file(o.clone());
    }
    validate_context(&vctx).map_err(|e| e.to_string())?;

    // Load input
    let mut codec = Codec::new();
    if let Err(e) = codec.read_file_by_extension(&input, lang.clone()) {
        // Graceful hint for unsupported or custom formats
        let ext = Path::new(&input).extension().and_then(|s| s.to_str()).unwrap_or("");
        if matches!(ext, "json" | "yaml" | "yml" | "langcodec") {
            return Err("Edit currently supports standard formats (.strings, .xml, .xcstrings, .csv, .tsv)".to_string());
        }
        return Err(format!("Failed to read input: {}", e));
    }

    // Determine operation: remove if value is None or Some("")
    let is_remove = value.as_deref().map(|s| s.is_empty()).unwrap_or(true);
    let status_parsed = parse_status_opt(&status)?;

    if is_remove {
        // Remove entry if present; treat missing as a no-op
        if let Some(ref l) = lang {
            if codec.has_entry(&key, l) {
                codec
                    .remove_entry(&key, l)
                    .map_err(|e| format!("{}", e))?;
                println!("✅ Removed '{}' from {}", key, l);
            } else {
                println!("ℹ️  Key '{}' not found in {}; nothing to remove", key, l);
            }
        } else {
            // No language specified: allow removal only if single resource present
            let res = pick_single_resource_mut(&mut codec, &lang)?;
            let before = res.entries.len();
            res.entries.retain(|e| e.id != key);
            if res.entries.len() < before {
                println!("✅ Removed '{}'", key);
            } else {
                println!("ℹ️  Key '{}' not present; nothing to remove", key);
            }
        }
    } else {
        // Add or update
        let lref = lang.as_deref().ok_or_else(|| "--lang is required for set on multi-language files".to_string())?;
        let val = value.unwrap_or_default();
        let exists = codec.has_entry(&key, lref);
        if exists {
            codec
                .update_translation(&key, lref, Translation::Singular(val.clone()), status_parsed.clone())
                .map_err(|e| format!("{}", e))?;
            if comment.is_some() {
                if let Some(entry) = codec.find_entry_mut(&key, lref) {
                    entry.comment = comment.clone();
                }
            }
            println!("✅ Updated '{}' in {}", key, lref);
        } else {
            codec
                .add_entry(&key, lref, Translation::Singular(val.clone()), comment.clone(), status_parsed.clone())
                .map_err(|e| format!("{}", e))?;
            println!("✅ Added '{}' to {}", key, lref);
        }
    }

    // Write back
    write_back(&codec, &input, &output, &lang)?;
    if let Some(out) = &output {
        println!("📄 Wrote changes to {}", out);
    } else {
        println!("📄 Updated {} in place", input);
    }

    Ok(())
}
