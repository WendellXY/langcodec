//! High-level resource operations (sync/diff) reusable by CLI and library users.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::{
    Error,
    provenance::{ProvenanceRecord, set_entry_provenance},
    types::{Resource, Translation},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchState {
    Exact,
    Fallback,
    Unmatched,
    Ambiguous,
}

impl MatchState {
    fn as_str(self) -> &'static str {
        match self {
            MatchState::Exact => "exact_key",
            MatchState::Fallback => "fallback_translation",
            MatchState::Unmatched => "unmatched",
            MatchState::Ambiguous => "ambiguous",
        }
    }
}

/// Issue type captured during sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncIssueKind {
    Unmatched,
    Ambiguous,
    MissingLanguage,
    TypeMismatch,
}

/// Per-entry sync issue details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncIssue {
    pub kind: SyncIssueKind,
    pub language: String,
    pub target_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_key: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub candidates: Vec<String>,
}

/// Options controlling sync behavior.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncOptions {
    pub language_filter: Option<String>,
    pub match_language: Option<String>,
    pub fail_on_unmatched: bool,
    pub fail_on_ambiguous: bool,
    pub record_provenance: bool,
}

/// Sync report with counters and issues.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SyncReport {
    pub match_language: String,
    pub processed_languages: usize,
    pub total_entries: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub fallback_matches: usize,
    pub skipped_unmatched: usize,
    pub skipped_missing_language: usize,
    pub skipped_ambiguous_fallback: usize,
    pub skipped_type_mismatch: usize,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub issues: Vec<SyncIssue>,
}

type SourceValues = HashMap<String, HashMap<String, Translation>>;

/// Updates entries in `target` using values from `source`.
///
/// Rules:
/// - Only existing entries in `target` are considered.
/// - Match by key first.
/// - Fallback match by translation in `match_language`.
/// - Never adds new keys to `target`.
pub fn sync_existing_entries(
    source: &[Resource],
    target: &mut [Resource],
    options: &SyncOptions,
) -> Result<SyncReport, Error> {
    let source_values = build_source_values(source);
    let match_language = infer_match_language(source, options.match_language.as_deref());
    let alias_map = build_alias_map(source, &match_language);
    let target_match_tokens = build_target_match_token_map(target, &match_language);

    let mut report = SyncReport {
        match_language,
        ..SyncReport::default()
    };

    for resource in target.iter_mut() {
        if let Some(filter_lang) = options.language_filter.as_deref()
            && !lang_matches(&resource.metadata.language, filter_lang)
        {
            continue;
        }

        report.processed_languages += 1;
        let target_lang = resource.metadata.language.clone();

        for entry in &mut resource.entries {
            report.total_entries += 1;
            let target_key = entry.id.clone();
            let target_match_token = target_match_tokens.get(&target_key).map(String::as_str);

            let (source_key_opt, match_state, candidates) = find_source_key_for_target(
                &target_key,
                &target_lang,
                target_match_token,
                &source_values,
                &alias_map,
            );

            let Some(source_key) = source_key_opt else {
                match match_state {
                    MatchState::Ambiguous => {
                        report.skipped_ambiguous_fallback += 1;
                        report.issues.push(SyncIssue {
                            kind: SyncIssueKind::Ambiguous,
                            language: target_lang.clone(),
                            target_key: target_key.clone(),
                            source_key: None,
                            candidates,
                        });
                    }
                    _ => {
                        report.skipped_unmatched += 1;
                        report.issues.push(SyncIssue {
                            kind: SyncIssueKind::Unmatched,
                            language: target_lang.clone(),
                            target_key: target_key.clone(),
                            source_key: None,
                            candidates: Vec::new(),
                        });
                    }
                }
                continue;
            };

            let Some(source_value) =
                source_value_for_lang(&source_values, &source_key, &target_lang)
            else {
                report.skipped_missing_language += 1;
                report.issues.push(SyncIssue {
                    kind: SyncIssueKind::MissingLanguage,
                    language: target_lang.clone(),
                    target_key: target_key.clone(),
                    source_key: Some(source_key),
                    candidates: Vec::new(),
                });
                continue;
            };

            let mut updated = false;
            match (&entry.value, source_value) {
                (Translation::Singular(current), Translation::Singular(new_value)) => {
                    if current != new_value {
                        entry.value = Translation::Singular(new_value.clone());
                        updated = true;
                    }
                }
                (Translation::Plural(current), Translation::Plural(new_value)) => {
                    if current != new_value {
                        entry.value = Translation::Plural(new_value.clone());
                        updated = true;
                    }
                }
                (Translation::Empty, Translation::Empty) => {}
                _ => {
                    report.skipped_type_mismatch += 1;
                    report.issues.push(SyncIssue {
                        kind: SyncIssueKind::TypeMismatch,
                        language: target_lang.clone(),
                        target_key: target_key.clone(),
                        source_key: Some(source_key),
                        candidates: Vec::new(),
                    });
                    continue;
                }
            }

            if updated {
                report.updated += 1;
                if match_state == MatchState::Fallback {
                    report.fallback_matches += 1;
                }
                if options.record_provenance {
                    let record = ProvenanceRecord {
                        source_language: Some(target_lang.clone()),
                        match_strategy: Some(match_state.as_str().to_string()),
                        source_key: Some(source_key),
                        ..ProvenanceRecord::default()
                    };
                    set_entry_provenance(entry, &record);
                }
            } else {
                report.unchanged += 1;
            }
        }
    }

    if options.fail_on_ambiguous
        && let Some(ambiguous) = report
            .issues
            .iter()
            .find(|issue| issue.kind == SyncIssueKind::Ambiguous)
    {
        return Err(Error::AmbiguousMatch {
            key: ambiguous.target_key.clone(),
            language: ambiguous.language.clone(),
            candidates: ambiguous.candidates.clone(),
        });
    }

    if options.fail_on_unmatched && report.skipped_unmatched > 0 {
        return Err(Error::policy_violation(format!(
            "sync has {} unmatched entries",
            report.skipped_unmatched
        )));
    }

    Ok(report)
}

/// Options controlling diff behavior.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffOptions {
    pub language_filter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffChangedItem {
    pub key: String,
    pub source: Translation,
    pub target: Translation,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LanguageDiff {
    pub language: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<DiffChangedItem>,
    pub unchanged: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DiffSummary {
    pub languages: usize,
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub unchanged: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DiffReport {
    pub summary: DiffSummary,
    pub languages: Vec<LanguageDiff>,
}

/// Produces an added/removed/changed diff between two resource sets.
pub fn diff_resources(
    source: &[Resource],
    target: &[Resource],
    options: &DiffOptions,
) -> DiffReport {
    let source_map = build_language_map(source);
    let target_map = build_language_map(target);

    let mut summary = DiffSummary::default();
    let mut languages = BTreeSet::new();
    languages.extend(source_map.keys().cloned());
    languages.extend(target_map.keys().cloned());

    let mut report_languages = Vec::new();
    for lang in languages {
        if let Some(filter) = options.language_filter.as_deref()
            && !lang_matches(&lang, filter)
        {
            continue;
        }

        let source_entries = source_map.get(&lang);
        let target_entries = target_map.get(&lang);
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
                (Some(s), Some(t)) if s != t => diff.changed.push(DiffChangedItem {
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
        report_languages.push(diff);
    }

    DiffReport {
        summary,
        languages: report_languages,
    }
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

fn singular_token(value: &Translation) -> Option<&str> {
    match value {
        Translation::Singular(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}

fn infer_match_language(source: &[Resource], explicit: Option<&str>) -> String {
    if let Some(lang) = explicit
        && !lang.trim().is_empty()
    {
        return lang.to_string();
    }

    if let Some(lang) = source
        .first()
        .and_then(|r| r.metadata.custom.get("source_language"))
        .filter(|s| !s.trim().is_empty())
    {
        return lang.clone();
    }

    if source
        .iter()
        .any(|r| normalize_lang(&r.metadata.language) == "en")
    {
        return "en".to_string();
    }

    source
        .first()
        .map(|r| r.metadata.language.clone())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "en".to_string())
}

fn pick_language_resource<'a>(resources: &'a [Resource], lang: &str) -> Option<&'a Resource> {
    resources
        .iter()
        .find(|r| lang_matches(&r.metadata.language, lang))
}

fn build_source_values(resources: &[Resource]) -> SourceValues {
    let mut values: SourceValues = HashMap::new();
    for resource in resources {
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

fn build_alias_map(resources: &[Resource], match_lang: &str) -> HashMap<String, Vec<String>> {
    let mut aliases: HashMap<String, Vec<String>> = HashMap::new();
    let Some(resource) = pick_language_resource(resources, match_lang) else {
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

fn build_target_match_token_map(
    resources: &[Resource],
    match_lang: &str,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(resource) = pick_language_resource(resources, match_lang) else {
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
) -> (Option<String>, Vec<String>, MatchState) {
    let Some(candidates) = alias_map.get(alias) else {
        return (None, Vec::new(), MatchState::Unmatched);
    };

    if candidates.len() == 1 {
        return (
            Some(candidates[0].clone()),
            Vec::new(),
            MatchState::Fallback,
        );
    }

    let matching_candidates: Vec<String> = candidates
        .iter()
        .filter(|key| source_value_for_lang(source_values, key, target_lang).is_some())
        .cloned()
        .collect();

    if matching_candidates.len() == 1 {
        return (
            Some(matching_candidates[0].clone()),
            Vec::new(),
            MatchState::Fallback,
        );
    }

    (None, matching_candidates, MatchState::Ambiguous)
}

fn find_source_key_for_target(
    target_key: &str,
    target_lang: &str,
    target_match_token: Option<&str>,
    source_values: &SourceValues,
    alias_map: &HashMap<String, Vec<String>>,
) -> (Option<String>, MatchState, Vec<String>) {
    if source_values.contains_key(target_key) {
        return (Some(target_key.to_string()), MatchState::Exact, Vec::new());
    }

    let (from_key, candidates, state) =
        resolve_alias_key_for_language(target_key, target_lang, alias_map, source_values);
    if from_key.is_some() || state == MatchState::Ambiguous {
        return (from_key, state, candidates);
    }

    if let Some(token) = target_match_token {
        let (from_token, token_candidates, token_state) =
            resolve_alias_key_for_language(token, target_lang, alias_map, source_values);
        if from_token.is_some() || token_state == MatchState::Ambiguous {
            return (from_token, token_state, token_candidates);
        }
    }

    (None, MatchState::Unmatched, Vec::new())
}

fn build_language_map(resources: &[Resource]) -> BTreeMap<String, BTreeMap<String, Translation>> {
    let mut map: BTreeMap<String, BTreeMap<String, Translation>> = BTreeMap::new();
    for resource in resources {
        let lang = normalize_lang(&resource.metadata.language);
        let entry_map = map.entry(lang).or_default();
        for entry in &resource.entries {
            entry_map.insert(entry.id.clone(), entry.value.clone());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::types::{Entry, EntryStatus, Metadata};

    use super::{DiffOptions, SyncIssueKind, SyncOptions, diff_resources, sync_existing_entries};

    fn entry(id: &str, value: &str) -> Entry {
        Entry {
            id: id.to_string(),
            value: crate::Translation::Singular(value.to_string()),
            comment: None,
            status: EntryStatus::Translated,
            custom: HashMap::new(),
        }
    }

    fn resource(language: &str, entries: Vec<Entry>) -> crate::Resource {
        crate::Resource {
            metadata: Metadata {
                language: language.to_string(),
                domain: "Localizable".to_string(),
                custom: HashMap::new(),
            },
            entries,
        }
    }

    #[test]
    fn test_sync_updates_existing_without_add() {
        let source = vec![resource(
            "en",
            vec![
                entry("hello", "Hello from source"),
                entry("added_only", "Should stay out"),
            ],
        )];
        let mut target = vec![resource(
            "en",
            vec![entry("hello", "Old"), entry("target_only", "Keep me")],
        )];

        let report = sync_existing_entries(&source, &mut target, &SyncOptions::default()).unwrap();
        assert_eq!(report.updated, 1);
        assert_eq!(report.skipped_unmatched, 1);
        assert_eq!(target[0].entries.len(), 2);
        assert_eq!(
            target[0]
                .entries
                .iter()
                .find(|e| e.id == "hello")
                .unwrap()
                .value,
            crate::Translation::Singular("Hello from source".to_string())
        );
    }

    #[test]
    fn test_sync_fallback_by_translation() {
        let source = vec![
            resource("en", vec![entry("welcome_title", "Welcome!")]),
            resource("fr", vec![entry("welcome_title", "Bienvenue!")]),
        ];
        let mut target = vec![
            resource("en", vec![entry("welcome", "Welcome!")]),
            resource("fr", vec![entry("welcome", "Ancienne bienvenue")]),
        ];

        let report = sync_existing_entries(
            &source,
            &mut target,
            &SyncOptions {
                record_provenance: true,
                ..SyncOptions::default()
            },
        )
        .unwrap();

        assert_eq!(report.fallback_matches, 1);
        let target_entry = target[1].entries.first().unwrap();
        assert_eq!(
            target_entry.value,
            crate::Translation::Singular("Bienvenue!".to_string())
        );
        assert_eq!(
            target_entry.custom.get("langcodec.provenance.source_key"),
            Some(&"welcome_title".to_string())
        );
    }

    #[test]
    fn test_sync_fail_on_ambiguous() {
        let source = vec![resource(
            "en",
            vec![
                entry("welcome_1", "Welcome!"),
                entry("welcome_2", "Welcome!"),
            ],
        )];
        let mut target = vec![resource("en", vec![entry("welcome", "Welcome!")])];
        let err = sync_existing_entries(
            &source,
            &mut target,
            &SyncOptions {
                fail_on_ambiguous: true,
                ..SyncOptions::default()
            },
        )
        .unwrap_err();
        assert_eq!(err.error_code(), crate::error::ErrorCode::AmbiguousMatch);
    }

    #[test]
    fn test_diff_counts() {
        let source = vec![resource(
            "en",
            vec![entry("same", "A"), entry("added", "B")],
        )];
        let target = vec![resource(
            "en",
            vec![entry("same", "A"), entry("removed", "C")],
        )];

        let report = diff_resources(&source, &target, &DiffOptions::default());
        assert_eq!(report.summary.languages, 1);
        assert_eq!(report.summary.added, 1);
        assert_eq!(report.summary.removed, 1);
        assert_eq!(report.summary.changed, 0);
        assert_eq!(report.summary.unchanged, 1);
    }

    #[test]
    fn test_sync_issue_kind_serialization() {
        let issue = SyncIssueKind::TypeMismatch;
        let encoded = serde_json::to_string(&issue).unwrap();
        assert_eq!(encoded, "\"type_mismatch\"");
    }
}
