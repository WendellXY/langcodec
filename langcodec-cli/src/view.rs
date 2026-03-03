use langcodec::{
    Codec,
    types::{EntryStatus, PluralCategory, Translation},
};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub struct ViewOptions {
    pub full: bool,
    pub status: Option<String>,
    pub keys_only: bool,
    pub json: bool,
}

const ACCEPTED_STATUSES: [&str; 5] = [
    "translated",
    "needs_review",
    "stale",
    "new",
    "do_not_translate",
];

fn parse_status_filter(status: &Option<String>) -> Result<Option<Vec<EntryStatus>>, String> {
    let Some(raw_status) = status else {
        return Ok(None);
    };

    let mut parsed = Vec::new();
    for token in raw_status.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let normalized = token.replace(['-', ' '], "_");
        let entry_status = normalized.parse::<EntryStatus>().map_err(|_| {
            format!(
                "Invalid status '{}'. Accepted statuses: {}",
                token,
                ACCEPTED_STATUSES.join(", ")
            )
        })?;

        if !parsed.contains(&entry_status) {
            parsed.push(entry_status);
        }
    }

    if parsed.is_empty() {
        return Err(format!(
            "No valid statuses were provided. Accepted statuses: {}",
            ACCEPTED_STATUSES.join(", ")
        ));
    }

    Ok(Some(parsed))
}

/// Truncate a string by Unicode scalar values (chars),
/// appending ellipsis if content exceeds `max_chars`.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let mut iter = s.chars();
    let truncated: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

fn status_label(status: &EntryStatus) -> &'static str {
    match status {
        EntryStatus::DoNotTranslate => "do_not_translate",
        EntryStatus::New => "new",
        EntryStatus::Stale => "stale",
        EntryStatus::NeedsReview => "needs_review",
        EntryStatus::Translated => "translated",
    }
}

fn plural_category_label(category: &PluralCategory) -> &'static str {
    match category {
        PluralCategory::Zero => "zero",
        PluralCategory::One => "one",
        PluralCategory::Two => "two",
        PluralCategory::Few => "few",
        PluralCategory::Many => "many",
        PluralCategory::Other => "other",
    }
}

fn render_json_output(
    filtered_resources: &[(&langcodec::Resource, Vec<&langcodec::types::Entry>)],
    lang_filter: &Option<String>,
    keys_only: bool,
) -> Result<String, String> {
    let mut total_matches = 0usize;
    let mut languages = BTreeSet::new();
    let mut status_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut entries_payload = Vec::new();
    let mut keys_by_language: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut keys_for_lang = Vec::new();

    for (resource, entries) in filtered_resources {
        languages.insert(resource.metadata.language.clone());

        for entry in entries {
            total_matches += 1;
            let status = status_label(&entry.status).to_string();
            *status_counts.entry(status.clone()).or_insert(0) += 1;

            if keys_only {
                if lang_filter.is_some() {
                    keys_for_lang.push(entry.id.clone());
                } else {
                    keys_by_language
                        .entry(resource.metadata.language.clone())
                        .or_default()
                        .push(entry.id.clone());
                }
                continue;
            }

            let mut entry_json = Map::new();
            entry_json.insert("lang".to_string(), json!(resource.metadata.language));
            entry_json.insert("key".to_string(), json!(entry.id));
            entry_json.insert("status".to_string(), json!(status));
            entry_json.insert("domain".to_string(), json!(resource.metadata.domain));

            match &entry.value {
                Translation::Empty => {
                    entry_json.insert("type".to_string(), json!("empty"));
                }
                Translation::Singular(value) => {
                    entry_json.insert("type".to_string(), json!("singular"));
                    entry_json.insert("value".to_string(), json!(value));
                }
                Translation::Plural(plural) => {
                    entry_json.insert("type".to_string(), json!("plural"));
                    entry_json.insert("plural_id".to_string(), json!(plural.id));
                    let mut forms = Map::new();
                    for (category, value) in &plural.forms {
                        forms.insert(plural_category_label(category).to_string(), json!(value));
                    }
                    entry_json.insert("forms".to_string(), Value::Object(forms));
                }
            }

            if let Some(comment) = &entry.comment {
                entry_json.insert("comment".to_string(), json!(comment));
            }

            entries_payload.push(Value::Object(entry_json));
        }
    }

    let summary = json!({
        "total_matches": total_matches,
        "languages": languages.into_iter().collect::<Vec<_>>(),
        "statuses": status_counts,
    });

    let payload = if keys_only {
        if lang_filter.is_some() {
            json!({
                "summary": summary,
                "keys": keys_for_lang,
            })
        } else {
            json!({
                "summary": summary,
                "keys": keys_by_language,
            })
        }
    } else {
        json!({
            "summary": summary,
            "entries": entries_payload,
        })
    };

    serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("Failed to render view JSON payload: {e}"))
}

/// Print a view of the resources in a codec.
pub fn print_view(codec: &Codec, lang_filter: &Option<String>, opts: &ViewOptions) {
    let keys_only_text = opts.keys_only && !opts.json;
    if !keys_only_text && !opts.json {
        println!("Processing resources...");
    }
    let status_filter = match parse_status_filter(&opts.status) {
        Ok(filter) => filter,
        Err(err) => {
            eprintln!("❌ {}", err);
            std::process::exit(1);
        }
    };

    // Use the new high-level methods from the lib crate
    let resources = if let Some(lang) = lang_filter {
        // Check if the language exists
        if !codec.languages().any(|l| l == lang) {
            println!("❌ Language not found");
            eprintln!(
                "Language '{}' not found. Available languages: {}",
                lang,
                codec.languages().collect::<Vec<_>>().join(", ")
            );
            std::process::exit(1);
        }

        // Get resources for the specific language
        codec
            .resources
            .iter()
            .filter(|r| r.metadata.language == *lang)
            .collect::<Vec<_>>()
    } else {
        // Get all resources
        codec.resources.iter().collect::<Vec<_>>()
    };

    if resources.is_empty() {
        println!("❌ No resources found");
        if let Some(lang) = lang_filter {
            eprintln!("No resources found for language: {}", lang);
        } else {
            eprintln!("No resources found");
        }
        std::process::exit(1);
    }

    if !keys_only_text && !opts.json {
        println!("✅ Found {} resource(s)", resources.len());
    }

    let filtered_resources = resources
        .iter()
        .map(|resource| {
            let entries = resource
                .entries
                .iter()
                .filter(|entry| {
                    status_filter
                        .as_ref()
                        .is_none_or(|statuses| statuses.contains(&entry.status))
                })
                .collect::<Vec<_>>();
            (*resource, entries)
        })
        .collect::<Vec<_>>();

    if opts.json {
        let rendered = match render_json_output(&filtered_resources, lang_filter, opts.keys_only) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("❌ {}", err);
                std::process::exit(1);
            }
        };
        println!("{}", rendered);
        return;
    }

    if keys_only_text {
        let include_lang_prefix = lang_filter.is_none();
        for (resource, entries) in &filtered_resources {
            for entry in entries {
                if include_lang_prefix {
                    println!("{}\t{}", resource.metadata.language, entry.id);
                } else {
                    println!("{}", entry.id);
                }
            }
        }
        return;
    }

    for (i, (resource, entries)) in filtered_resources.iter().enumerate() {
        println!("\n=== Resource {} ===", i + 1);
        println!("Language: {}", resource.metadata.language);
        println!("Domain: {}", resource.metadata.domain);
        println!("Entries: {}", entries.len());

        for (j, entry) in entries.iter().enumerate() {
            println!("\n  Entry {}: {}", j + 1, entry.id);
            println!("    Status: {:?}", entry.status);

            if let Some(comment) = &entry.comment {
                println!("    Comment: {}", comment);
            }

            match &entry.value {
                Translation::Empty => {
                    println!("    Type: Empty");
                }
                Translation::Singular(value) => {
                    println!("    Type: Singular");
                    if opts.full {
                        println!("    Value: {}", value);
                    } else {
                        let truncated = truncate_chars(value, 50);
                        println!("    Value: {}", truncated);
                    }
                }
                Translation::Plural(plural) => {
                    println!("    Type: Plural");
                    println!("    Plural ID: {}", plural.id);
                    for (category, value) in &plural.forms {
                        if opts.full {
                            println!("      {:?}: {}", category, value);
                        } else {
                            let truncated = truncate_chars(value, 50);
                            println!("      {:?}: {}", category, truncated);
                        }
                    }
                }
            }
        }
    }

    // Show summary using the new high-level methods
    if lang_filter.is_none() {
        let mut unique_keys = HashSet::new();
        let mut per_language_counts: BTreeMap<String, usize> = BTreeMap::new();
        for (resource, entries) in &filtered_resources {
            per_language_counts
                .entry(resource.metadata.language.clone())
                .and_modify(|count| *count += entries.len())
                .or_insert(entries.len());
            for entry in entries {
                unique_keys.insert(entry.id.clone());
            }
        }

        println!("\n=== Summary ===");
        println!("Total languages: {}", per_language_counts.len());
        println!("Total unique keys: {}", unique_keys.len());

        for (lang, count) in per_language_counts {
            println!("  {}: {} entries", lang, count);
        }
    }
}
