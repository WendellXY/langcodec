use crate::convert::read_resources_from_any_input;
use crate::validation::{validate_file_path, validate_language_code, validate_output_path};
use langcodec::{Codec, Resource, Translation, formats::FormatType};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SyncOptions {
    pub source: String,
    pub target: String,
    pub output: Option<String>,
    pub lang: Option<String>,
    pub match_lang: Option<String>,
    pub report_json: Option<String>,
    pub fail_on_unmatched: bool,
    pub fail_on_ambiguous: bool,
    pub strict: bool,
    pub dry_run: bool,
}

#[derive(Debug, Default)]
struct SyncStats {
    total_entries: usize,
    updated: usize,
    unchanged: usize,
    fallback_matches: usize,
    skipped_unmatched: usize,
    skipped_missing_language: usize,
    skipped_ambiguous_fallback: usize,
    skipped_type_mismatch: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchState {
    Exact,
    Fallback,
    Unmatched,
    Ambiguous,
}

type SourceValues = HashMap<String, HashMap<String, Translation>>;

fn normalize_lang(lang: &str) -> String {
    lang.trim().replace('_', "-").to_ascii_lowercase()
}

fn lang_base(lang: &str) -> &str {
    lang.split('-').next().unwrap_or(lang)
}

fn lang_matches(resource_lang: &str, requested_lang: &str) -> bool {
    let res = normalize_lang(resource_lang);
    let req = normalize_lang(requested_lang);
    res == req || lang_base(&res) == lang_base(&req)
}

fn infer_match_language(source: &Codec, explicit: &Option<String>) -> String {
    if let Some(lang) = explicit
        && !lang.trim().is_empty()
    {
        return lang.clone();
    }

    if let Some(lang) = source
        .resources
        .first()
        .and_then(|r| r.metadata.custom.get("source_language"))
        .filter(|s| !s.trim().is_empty())
    {
        return lang.clone();
    }

    if source
        .resources
        .iter()
        .any(|r| normalize_lang(&r.metadata.language) == "en")
    {
        return "en".to_string();
    }

    source
        .resources
        .first()
        .map(|r| r.metadata.language.clone())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "en".to_string())
}

fn pick_language_resource<'a>(codec: &'a Codec, lang: &str) -> Option<&'a Resource> {
    codec
        .resources
        .iter()
        .find(|r| lang_matches(&r.metadata.language, lang))
}

fn singular_token(value: &Translation) -> Option<&str> {
    match value {
        Translation::Singular(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}

fn build_source_values(codec: &Codec) -> SourceValues {
    let mut values: SourceValues = HashMap::new();
    for resource in &codec.resources {
        let lang = normalize_lang(&resource.metadata.language);
        for entry in &resource.entries {
            values
                .entry(entry.id.clone())
                .or_default()
                .insert(lang.clone(), entry.value.clone());
        }
    }
    values
}

fn source_value_for_lang<'a>(
    values: &'a SourceValues,
    key: &str,
    target_lang: &str,
) -> Option<&'a Translation> {
    let by_lang = values.get(key)?;
    let target_norm = normalize_lang(target_lang);
    if let Some(v) = by_lang.get(&target_norm) {
        return Some(v);
    }
    let target_base = lang_base(&target_norm);
    let mut base_matches = by_lang
        .iter()
        .filter(|(lang, _)| lang_base(lang.as_str()) == target_base);
    let first = base_matches.next().map(|(_, v)| v)?;
    if base_matches.next().is_some() {
        return None;
    }
    Some(first)
}

fn build_alias_map(source: &Codec, match_lang: &str) -> HashMap<String, Vec<String>> {
    let mut aliases: HashMap<String, Vec<String>> = HashMap::new();
    let Some(resource) = pick_language_resource(source, match_lang) else {
        return aliases;
    };

    for entry in &resource.entries {
        let Some(token) = singular_token(&entry.value) else {
            continue;
        };
        let keys = aliases.entry(token.to_string()).or_default();
        if !keys.iter().any(|k| k == &entry.id) {
            keys.push(entry.id.clone());
        }
    }

    aliases
}

fn build_target_match_token_map(target: &Codec, match_lang: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(resource) = pick_language_resource(target, match_lang) else {
        return map;
    };

    for entry in &resource.entries {
        if let Some(token) = singular_token(&entry.value) {
            map.insert(entry.id.clone(), token.to_string());
        }
    }
    map
}

fn resolve_alias_key_for_language(
    alias: &str,
    target_lang: &str,
    alias_map: &HashMap<String, Vec<String>>,
    source_values: &SourceValues,
) -> Result<Option<String>, ()> {
    let Some(candidates) = alias_map.get(alias) else {
        return Ok(None);
    };

    if candidates.len() == 1 {
        return Ok(Some(candidates[0].clone()));
    }

    let mut language_candidates = candidates
        .iter()
        .filter(|key| source_value_for_lang(source_values, key, target_lang).is_some());

    let first = language_candidates.next();
    let second = language_candidates.next();
    match (first, second) {
        (Some(k), None) => Ok(Some(k.clone())),
        (_, Some(_)) => Err(()),
        _ => Err(()),
    }
}

fn find_source_key_for_target(
    target_key: &str,
    target_lang: &str,
    target_match_token: Option<&str>,
    source_values: &SourceValues,
    alias_map: &HashMap<String, Vec<String>>,
) -> (Option<String>, MatchState) {
    if source_values.contains_key(target_key) {
        return (Some(target_key.to_string()), MatchState::Exact);
    }

    match resolve_alias_key_for_language(target_key, target_lang, alias_map, source_values) {
        Ok(Some(k)) => return (Some(k), MatchState::Fallback),
        Err(()) => return (None, MatchState::Ambiguous),
        Ok(None) => {}
    }

    if let Some(token) = target_match_token {
        match resolve_alias_key_for_language(token, target_lang, alias_map, source_values) {
            Ok(Some(k)) => return (Some(k), MatchState::Fallback),
            Err(()) => return (None, MatchState::Ambiguous),
            Ok(None) => {}
        }
    }

    (None, MatchState::Unmatched)
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
            .resources
            .iter()
            .find(|r| lang_matches(&r.metadata.language, l))
            .ok_or_else(|| format!("Language '{}' not found in output resources", l))
    } else if codec.resources.len() == 1 {
        Ok(&codec.resources[0])
    } else {
        Err("Multiple languages present; specify --lang".to_string())
    }
}

fn write_back(
    codec: &Codec,
    target_path: &str,
    output_path: &Option<String>,
    lang: &Option<String>,
) -> Result<(), String> {
    let target_owned = target_path.to_string();
    let out = output_path.as_ref().unwrap_or(&target_owned);
    let fmt = infer_output_format_from_path(out)?;

    match fmt {
        FormatType::Strings(_) | FormatType::AndroidStrings(_) => {
            let res = pick_single_resource(codec, lang)?;
            langcodec::Codec::write_resource_to_file(res, out)
                .map_err(|e| format!("Error writing output: {}", e))
        }
        FormatType::Xcstrings | FormatType::CSV | FormatType::TSV => {
            langcodec::converter::convert_resources_to_format(codec.resources.clone(), out, fmt)
                .map_err(|e| format!("Error writing output: {}", e))
        }
    }
}

fn write_report(
    path: &str,
    options: &SyncOptions,
    match_lang: &str,
    stats: &SyncStats,
) -> Result<(), String> {
    let report = json!({
        "source": options.source,
        "target": options.target,
        "output": options.output,
        "lang": options.lang,
        "match_lang": match_lang,
        "strict": options.strict,
        "fail_on_unmatched": options.fail_on_unmatched,
        "fail_on_ambiguous": options.fail_on_ambiguous,
        "dry_run": options.dry_run,
        "summary": {
            "total_entries": stats.total_entries,
            "updated": stats.updated,
            "unchanged": stats.unchanged,
            "fallback_matches": stats.fallback_matches,
            "skipped_unmatched": stats.skipped_unmatched,
            "skipped_missing_language": stats.skipped_missing_language,
            "skipped_ambiguous_fallback": stats.skipped_ambiguous_fallback,
            "skipped_type_mismatch": stats.skipped_type_mismatch
        }
    });

    let text = serde_json::to_string_pretty(&report)
        .map_err(|e| format!("Failed to serialize report JSON: {}", e))?;
    std::fs::write(path, text).map_err(|e| format!("Failed to write report JSON '{}': {}", path, e))
}

pub fn run_sync_command(opts: SyncOptions) -> Result<(), String> {
    validate_file_path(&opts.source)?;
    validate_file_path(&opts.target)?;
    if let Some(output) = &opts.output {
        validate_output_path(output)?;
    }
    if let Some(lang) = &opts.lang {
        validate_language_code(lang)?;
    }
    if let Some(match_lang) = &opts.match_lang {
        validate_language_code(match_lang)?;
    }
    if let Some(report_path) = &opts.report_json {
        validate_output_path(report_path)?;
    }

    let source_resources = read_resources_from_any_input(&opts.source, None, opts.strict)?;
    let source_codec = Codec {
        resources: source_resources,
    };
    let mut target_codec = Codec::new();
    target_codec
        .read_file_by_extension(&opts.target, None)
        .map_err(|e| format!("Failed to read target '{}': {}", opts.target, e))?;

    let match_lang = infer_match_language(&source_codec, &opts.match_lang);
    let source_values = build_source_values(&source_codec);
    let alias_map = build_alias_map(&source_codec, &match_lang);
    let target_match_tokens = build_target_match_token_map(&target_codec, &match_lang);

    let mut stats = SyncStats::default();
    let mut processed_language_count = 0usize;

    for resource in &mut target_codec.resources {
        if let Some(filter_lang) = &opts.lang
            && !lang_matches(&resource.metadata.language, filter_lang)
        {
            continue;
        }

        processed_language_count += 1;
        let target_lang = resource.metadata.language.clone();

        for entry in &mut resource.entries {
            stats.total_entries += 1;

            let target_key = entry.id.clone();
            let target_match_token = target_match_tokens
                .get(&target_key)
                .map(std::string::String::as_str);

            let (source_key_opt, match_state) = find_source_key_for_target(
                &target_key,
                &target_lang,
                target_match_token,
                &source_values,
                &alias_map,
            );

            match match_state {
                MatchState::Ambiguous => {
                    stats.skipped_ambiguous_fallback += 1;
                    continue;
                }
                MatchState::Unmatched => {
                    stats.skipped_unmatched += 1;
                    continue;
                }
                MatchState::Exact | MatchState::Fallback => {}
            }

            let Some(source_key) = source_key_opt else {
                stats.skipped_unmatched += 1;
                continue;
            };

            let Some(source_value) =
                source_value_for_lang(&source_values, &source_key, &target_lang)
            else {
                stats.skipped_missing_language += 1;
                continue;
            };

            if std::mem::discriminant(&entry.value) != std::mem::discriminant(source_value) {
                stats.skipped_type_mismatch += 1;
                continue;
            }

            if entry.value == *source_value {
                stats.unchanged += 1;
                continue;
            }

            if match_state == MatchState::Fallback {
                stats.fallback_matches += 1;
            }

            if opts.dry_run {
                println!(
                    "DRY-RUN: [{}] '{}' <= source key '{}'",
                    target_lang, target_key, source_key
                );
            } else {
                entry.value = source_value.clone();
            }
            stats.updated += 1;
        }
    }

    if opts.lang.is_some() && processed_language_count == 0 {
        return Err(format!(
            "Language '{}' not found in target file",
            opts.lang.unwrap_or_default()
        ));
    }

    println!("Sync match language: {}", match_lang);
    println!("Total target entries considered: {}", stats.total_entries);
    println!("Updated: {}", stats.updated);
    println!("Unchanged: {}", stats.unchanged);
    println!("Fallback matches used: {}", stats.fallback_matches);
    println!("Skipped (unmatched): {}", stats.skipped_unmatched);
    println!(
        "Skipped (missing source language value): {}",
        stats.skipped_missing_language
    );
    println!(
        "Skipped (ambiguous fallback): {}",
        stats.skipped_ambiguous_fallback
    );
    println!("Skipped (type mismatch): {}", stats.skipped_type_mismatch);

    if let Some(report_path) = &opts.report_json {
        write_report(report_path, &opts, &match_lang, &stats)?;
        println!("Report JSON written: {}", report_path);
    }

    let fail_on_unmatched = opts.fail_on_unmatched || opts.strict;
    let fail_on_ambiguous = opts.fail_on_ambiguous || opts.strict;
    if (fail_on_unmatched && stats.skipped_unmatched > 0)
        || (fail_on_ambiguous && stats.skipped_ambiguous_fallback > 0)
    {
        let mut reasons = Vec::new();
        if fail_on_unmatched && stats.skipped_unmatched > 0 {
            reasons.push(format!("unmatched={}", stats.skipped_unmatched));
        }
        if fail_on_ambiguous && stats.skipped_ambiguous_fallback > 0 {
            reasons.push(format!("ambiguous={}", stats.skipped_ambiguous_fallback));
        }
        return Err(format!("Sync policy failure ({})", reasons.join(", ")));
    }

    if opts.dry_run {
        println!("Dry-run mode: no files were written");
        return Ok(());
    }

    write_back(&target_codec, &opts.target, &opts.output, &opts.lang)?;
    println!(
        "✅ Sync complete: {}",
        opts.output.as_deref().unwrap_or(&opts.target)
    );
    Ok(())
}
