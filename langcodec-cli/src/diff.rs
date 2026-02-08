use crate::convert::read_resources_from_any_input;
use crate::validation::{validate_file_path, validate_language_code, validate_output_path};
use langcodec::{Resource, Translation};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct DiffOptions {
    pub source: String,
    pub target: String,
    pub lang: Option<String>,
    pub json: bool,
    pub output: Option<String>,
    pub strict: bool,
}

#[derive(Debug, Clone)]
struct ChangedItem {
    key: String,
    source: Translation,
    target: Translation,
}

#[derive(Debug, Clone, Default)]
struct LanguageDiff {
    language: String,
    added: Vec<String>,
    removed: Vec<String>,
    changed: Vec<ChangedItem>,
    unchanged: usize,
}

#[derive(Debug, Clone, Default)]
struct DiffSummary {
    languages: usize,
    added: usize,
    removed: usize,
    changed: usize,
    unchanged: usize,
}

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

fn translation_as_text(value: &Translation) -> String {
    match value {
        Translation::Empty => String::new(),
        Translation::Singular(v) => v.clone(),
        Translation::Plural(p) => {
            let mut parts = Vec::new();
            for (category, text) in &p.forms {
                parts.push(format!("{:?}={}", category, text));
            }
            parts.join(" | ")
        }
    }
}

fn build_language_map(resources: Vec<Resource>) -> BTreeMap<String, BTreeMap<String, Translation>> {
    let mut map: BTreeMap<String, BTreeMap<String, Translation>> = BTreeMap::new();
    for resource in resources {
        let lang = normalize_lang(&resource.metadata.language);
        let entry_map = map.entry(lang).or_default();
        for entry in resource.entries {
            entry_map.insert(entry.id, entry.value);
        }
    }
    map
}

fn collect_diff(
    source: &BTreeMap<String, BTreeMap<String, Translation>>,
    target: &BTreeMap<String, BTreeMap<String, Translation>>,
    lang_filter: &Option<String>,
) -> (DiffSummary, Vec<LanguageDiff>) {
    let mut summary = DiffSummary::default();
    let mut languages = BTreeSet::new();
    languages.extend(source.keys().cloned());
    languages.extend(target.keys().cloned());

    let mut per_language = Vec::new();
    for lang in languages {
        if let Some(filter) = lang_filter
            && !lang_matches(&lang, filter)
        {
            continue;
        }

        let source_entries = source.get(&lang);
        let target_entries = target.get(&lang);

        let mut all_keys = BTreeSet::new();
        if let Some(entries) = source_entries {
            all_keys.extend(entries.keys().cloned());
        }
        if let Some(entries) = target_entries {
            all_keys.extend(entries.keys().cloned());
        }

        let mut diff = LanguageDiff {
            language: lang.clone(),
            ..LanguageDiff::default()
        };

        for key in all_keys {
            let source_value = source_entries.and_then(|m| m.get(&key));
            let target_value = target_entries.and_then(|m| m.get(&key));
            match (source_value, target_value) {
                (Some(_), None) => diff.added.push(key),
                (None, Some(_)) => diff.removed.push(key),
                (Some(s), Some(t)) if s != t => diff.changed.push(ChangedItem {
                    key,
                    source: s.clone(),
                    target: t.clone(),
                }),
                (Some(_), Some(_)) => diff.unchanged += 1,
                (None, None) => {}
            }
        }

        summary.languages += 1;
        summary.added += diff.added.len();
        summary.removed += diff.removed.len();
        summary.changed += diff.changed.len();
        summary.unchanged += diff.unchanged;
        per_language.push(diff);
    }

    (summary, per_language)
}

fn print_or_write(output: Option<&String>, content: &str) -> Result<(), String> {
    if let Some(path) = output {
        std::fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path, e))?;
        println!("Report written: {}", path);
    } else {
        println!("{}", content);
    }
    Ok(())
}

fn render_human(summary: &DiffSummary, per_language: &[LanguageDiff]) -> String {
    let mut lines = Vec::new();
    lines.push("=== Diff ===".to_string());
    lines.push(format!("Languages: {}", summary.languages));
    lines.push(format!(
        "Totals: added={}, removed={}, changed={}, unchanged={}",
        summary.added, summary.removed, summary.changed, summary.unchanged
    ));

    for lang in per_language {
        lines.push(format!("\nLanguage: {}", lang.language));
        lines.push(format!("  added: {}", lang.added.len()));
        lines.push(format!("  removed: {}", lang.removed.len()));
        lines.push(format!("  changed: {}", lang.changed.len()));
        lines.push(format!("  unchanged: {}", lang.unchanged));
        if !lang.added.is_empty() {
            lines.push(format!("  added keys: {}", lang.added.join(", ")));
        }
        if !lang.removed.is_empty() {
            lines.push(format!("  removed keys: {}", lang.removed.join(", ")));
        }
        if !lang.changed.is_empty() {
            let mut changed_lines = Vec::new();
            for item in &lang.changed {
                changed_lines.push(format!(
                    "{} ('{}' -> '{}')",
                    item.key,
                    translation_as_text(&item.target),
                    translation_as_text(&item.source)
                ));
            }
            lines.push(format!("  changed keys: {}", changed_lines.join(", ")));
        }
    }

    lines.join("\n")
}

fn render_json(summary: &DiffSummary, per_language: &[LanguageDiff]) -> Result<String, String> {
    let languages_json: Vec<_> = per_language
        .iter()
        .map(|lang| {
            let changed: Vec<_> = lang
                .changed
                .iter()
                .map(|item| {
                    json!({
                        "key": item.key,
                        "source": item.source,
                        "target": item.target,
                    })
                })
                .collect();

            json!({
                "language": lang.language,
                "counts": {
                    "added": lang.added.len(),
                    "removed": lang.removed.len(),
                    "changed": lang.changed.len(),
                    "unchanged": lang.unchanged,
                },
                "added": lang.added,
                "removed": lang.removed,
                "changed": changed,
            })
        })
        .collect();

    let report = json!({
        "summary": {
            "languages": summary.languages,
            "added": summary.added,
            "removed": summary.removed,
            "changed": summary.changed,
            "unchanged": summary.unchanged,
        },
        "languages": languages_json,
    });

    serde_json::to_string_pretty(&report)
        .map_err(|e| format!("Failed to serialize diff report JSON: {}", e))
}

pub fn run_diff_command(opts: DiffOptions) -> Result<(), String> {
    validate_file_path(&opts.source)?;
    validate_file_path(&opts.target)?;
    if let Some(lang) = &opts.lang {
        validate_language_code(lang)?;
    }
    if let Some(output) = &opts.output {
        validate_output_path(output)?;
    }

    let source_resources = read_resources_from_any_input(&opts.source, None, opts.strict)?;
    let target_resources = read_resources_from_any_input(&opts.target, None, opts.strict)?;

    let source_map = build_language_map(source_resources);
    let target_map = build_language_map(target_resources);
    let (summary, per_language) = collect_diff(&source_map, &target_map, &opts.lang);

    if opts.json {
        let rendered = render_json(&summary, &per_language)?;
        print_or_write(opts.output.as_ref(), &rendered)?;
    } else {
        let rendered = render_human(&summary, &per_language);
        print_or_write(opts.output.as_ref(), &rendered)?;
    }

    Ok(())
}
