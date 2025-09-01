use langcodec::{collect_resource_plural_issues, Codec, types::EntryStatus};
use serde_json::json;
use std::collections::HashMap;

#[derive(Default)]
struct LangStats {
    total: usize,
    by_status: HashMap<&'static str, usize>,
    translated: usize,
    denominator: usize,
}

fn accumulate(lang_stats: &mut LangStats, status: &EntryStatus) {
    lang_stats.total += 1;
    let key: &'static str = match status {
        EntryStatus::DoNotTranslate => "do_not_translate",
        EntryStatus::New => "new",
        EntryStatus::Stale => "stale",
        EntryStatus::NeedsReview => "needs_review",
        EntryStatus::Translated => "translated",
    };
    *lang_stats.by_status.entry(key).or_insert(0) += 1;
    if matches!(status, EntryStatus::Translated) {
        lang_stats.translated += 1;
    }
    if !matches!(status, EntryStatus::DoNotTranslate) {
        lang_stats.denominator += 1;
    }
}

pub fn print_stats(codec: &Codec, lang_filter: &Option<String>, json_output: bool) {
    let resources: Vec<_> = match lang_filter {
        Some(lang) => codec
            .resources
            .iter()
            .filter(|r| r.metadata.language == *lang)
            .collect(),
        None => codec.resources.iter().collect(),
    };

    if json_output {
        // Build JSON object
        let mut per_lang = Vec::new();
        for res in &resources {
            let mut stats = LangStats::default();
            for e in &res.entries {
                accumulate(&mut stats, &e.status);
            }
            let plural_issues = collect_resource_plural_issues(res);
            let missing_plural_entries = plural_issues.len();
            let missing_plural_categories_total: usize =
                plural_issues.iter().map(|r| r.missing.len()).sum();
            let percent = if stats.denominator == 0 {
                100.0
            } else {
                (stats.translated as f64) * 100.0 / (stats.denominator as f64)
            };
            per_lang.push(json!({
                "language": res.metadata.language,
                "total": stats.total,
                "by_status": stats.by_status,
                "completion_percent": (percent * 100.0).round() / 100.0,
                "missing_plural_entries": missing_plural_entries,
                "missing_plural_categories_total": missing_plural_categories_total,
            }));
        }
        let summary = json!({
            "languages": resources.len(),
            "unique_keys": codec.all_keys().count(),
        });
        let body = json!({
            "summary": summary,
            "languages": per_lang,
        });
        println!("{}", serde_json::to_string_pretty(&body).unwrap());
        return;
    }

    println!("=== Stats ===");
    println!("Languages: {}", resources.len());
    println!("Unique keys: {}", codec.all_keys().count());

    for res in &resources {
        let mut stats = LangStats::default();
        for e in &res.entries {
            accumulate(&mut stats, &e.status);
        }
        let plural_issues = collect_resource_plural_issues(res);
        let missing_plural_entries = plural_issues.len();
        let missing_plural_categories_total: usize =
            plural_issues.iter().map(|r| r.missing.len()).sum();
        let percent = if stats.denominator == 0 {
            100.0
        } else {
            (stats.translated as f64) * 100.0 / (stats.denominator as f64)
        };
        println!("\nLanguage: {}", res.metadata.language);
        println!("  Total: {}", stats.total);
        println!("  By status:");
        for (k, v) in [
            (
                "translated",
                stats.by_status.get("translated").copied().unwrap_or(0),
            ),
            (
                "needs_review",
                stats.by_status.get("needs_review").copied().unwrap_or(0),
            ),
            ("stale", stats.by_status.get("stale").copied().unwrap_or(0)),
            ("new", stats.by_status.get("new").copied().unwrap_or(0)),
            (
                "do_not_translate",
                stats
                    .by_status
                    .get("do_not_translate")
                    .copied()
                    .unwrap_or(0),
            ),
        ] {
            println!("    {}: {}", k, v);
        }
        println!("  Completion: {:.2}%", percent);
        println!(
            "  Missing plurals: {} (missing categories: {})",
            missing_plural_entries, missing_plural_categories_total
        );
    }
}
