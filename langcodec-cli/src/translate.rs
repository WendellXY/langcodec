use crate::config::{LoadedConfig, load_config};
use crate::validation::{validate_language_code, validate_output_path};
use async_trait::async_trait;
use langcodec::{
    Codec, Entry, EntryStatus, FormatType, Metadata, ReadOptions, Resource, Translation,
    convert_resources_to_format, infer_format_from_extension, infer_language_from_path,
};
use mentra::provider::{
    self, ContentBlock, Message, Provider, ProviderError, ProviderRequestOptions, Request,
};
use serde::Deserialize;
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    runtime::Builder,
    sync::{Mutex as AsyncMutex, mpsc},
    task::JoinSet,
};

const DEFAULT_STATUSES: [&str; 2] = ["new", "stale"];
const DEFAULT_CONCURRENCY: usize = 4;
const SYSTEM_PROMPT: &str = "You translate application localization strings. Return JSON only with the shape {\"translation\":\"...\"}. Preserve placeholders, escapes, newline markers, surrounding punctuation, HTML/XML tags, Markdown, and product names exactly unless the target language grammar requires adjacent spacing changes. Never add explanations or extra keys.";

#[derive(Debug, Clone)]
pub struct TranslateOptions {
    pub source: String,
    pub target: Option<String>,
    pub output: Option<String>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub status: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub concurrency: Option<usize>,
    pub config: Option<String>,
    pub dry_run: bool,
    pub strict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProviderKind {
    OpenAI,
    Anthropic,
    Gemini,
}

impl ProviderKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" => Ok(Self::OpenAI),
            "anthropic" => Ok(Self::Anthropic),
            "gemini" => Ok(Self::Gemini),
            other => Err(format!(
                "Unsupported provider '{}'. Expected one of: openai, anthropic, gemini",
                other
            )),
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
        }
    }

    fn api_key_env(&self) -> &'static str {
        match self {
            Self::OpenAI => "OPENAI_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::Gemini => "GEMINI_API_KEY",
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedOptions {
    source: String,
    target: Option<String>,
    output: Option<String>,
    source_lang: Option<String>,
    target_lang: String,
    statuses: Vec<EntryStatus>,
    provider: ProviderKind,
    model: String,
    concurrency: usize,
    dry_run: bool,
    strict: bool,
}

#[derive(Debug, Clone)]
struct SelectedResource {
    language: String,
    resource: Resource,
}

#[derive(Debug, Clone)]
struct TranslationJob {
    key: String,
    source_lang: String,
    target_lang: String,
    source_value: String,
    source_comment: Option<String>,
    existing_comment: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct TranslationSummary {
    total_entries: usize,
    queued: usize,
    translated: usize,
    skipped_do_not_translate: usize,
    skipped_plural: usize,
    skipped_status: usize,
    skipped_empty_source: usize,
    failed: usize,
}

#[derive(Debug, Clone)]
struct TranslationResult {
    key: String,
    translated_value: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TranslateOutcome {
    pub translated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone)]
struct PreparedTranslation {
    opts: ResolvedOptions,
    source_path: String,
    target_path: String,
    output_path: String,
    output_format: FormatType,
    config_path: Option<PathBuf>,
    source_resource: SelectedResource,
    target_codec: Codec,
    jobs: Vec<TranslationJob>,
    summary: TranslationSummary,
}

#[derive(Clone)]
struct ProviderSetup {
    provider_kind: ProviderKind,
    provider: Arc<dyn Provider>,
}

#[derive(Clone)]
struct MentraBackend {
    provider: Arc<dyn Provider>,
    model: String,
}

#[derive(Debug, Clone)]
struct BackendRequest {
    key: String,
    source_lang: String,
    target_lang: String,
    source_value: String,
    source_comment: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelTranslationPayload {
    translation: String,
}

#[async_trait]
trait TranslationBackend: Send + Sync {
    async fn translate(&self, request: BackendRequest) -> Result<String, String>;
}

#[async_trait]
impl TranslationBackend for MentraBackend {
    async fn translate(&self, request: BackendRequest) -> Result<String, String> {
        let prompt = build_prompt(&request);
        let response = self
            .provider
            .send(Request {
                model: Cow::Borrowed(self.model.as_str()),
                system: Some(Cow::Borrowed(SYSTEM_PROMPT)),
                messages: Cow::Owned(vec![Message::user(ContentBlock::text(prompt))]),
                tools: Cow::Owned(Vec::new()),
                tool_choice: None,
                temperature: Some(0.2),
                max_output_tokens: Some(512),
                metadata: Cow::Owned(BTreeMap::new()),
                provider_request_options: ProviderRequestOptions::default(),
            })
            .await
            .map_err(format_provider_error)?;

        let text = collect_text_blocks(&response);
        parse_translation_response(&text)
    }
}

pub fn run_translate_command(opts: TranslateOptions) -> Result<TranslateOutcome, String> {
    let prepared = prepare_translation(&opts)?;
    let backend = create_mentra_backend(&prepared.opts)?;
    run_prepared_translation(prepared, Arc::new(backend))
}

fn run_prepared_translation(
    prepared: PreparedTranslation,
    backend: Arc<dyn TranslationBackend>,
) -> Result<TranslateOutcome, String> {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create async runtime: {}", e))?;
    runtime.block_on(async_run_translation(prepared, backend))
}

async fn async_run_translation(
    mut prepared: PreparedTranslation,
    backend: Arc<dyn TranslationBackend>,
) -> Result<TranslateOutcome, String> {
    print_preamble(&prepared);

    if prepared.jobs.is_empty() {
        print_summary(&prepared.summary);
        if prepared.opts.dry_run {
            println!("Dry-run mode: no files were written");
        } else {
            write_back(
                &prepared.target_codec,
                &prepared.output_path,
                &prepared.output_format,
                &prepared.opts.target_lang,
            )?;
            println!("✅ Translate complete: {}", prepared.output_path);
        }
        return Ok(TranslateOutcome {
            translated: 0,
            skipped: count_skipped(&prepared.summary),
            failed: 0,
            output_path: Some(prepared.output_path),
        });
    }

    let worker_count = prepared.opts.concurrency.min(prepared.jobs.len()).max(1);
    let queue = Arc::new(AsyncMutex::new(VecDeque::from(prepared.jobs.clone())));
    let (tx, mut rx) = mpsc::unbounded_channel::<Result<TranslationResult, String>>();
    let mut join_set = JoinSet::new();
    for _ in 0..worker_count {
        let backend = Arc::clone(&backend);
        let queue = Arc::clone(&queue);
        let tx = tx.clone();
        join_set.spawn(async move {
            loop {
                let job = {
                    let mut queue = queue.lock().await;
                    queue.pop_front()
                };

                let Some(job) = job else {
                    break;
                };

                let result = backend
                    .translate(BackendRequest {
                        key: job.key.clone(),
                        source_lang: job.source_lang.clone(),
                        target_lang: job.target_lang.clone(),
                        source_value: job.source_value.clone(),
                        source_comment: job.source_comment.clone(),
                    })
                    .await
                    .map(|translated_value| TranslationResult {
                        key: job.key.clone(),
                        translated_value,
                    });
                let _ = tx.send(result);
            }

            Ok::<(), String>(())
        });
    }
    drop(tx);

    let mut results: HashMap<String, String> = HashMap::new();
    let mut completed = 0usize;

    while let Some(result) = rx.recv().await {
        completed += 1;
        match result {
            Ok(item) => {
                prepared.summary.translated += 1;
                results.insert(item.key, item.translated_value);
            }
            Err(err) => {
                prepared.summary.failed += 1;
                eprintln!("✖ {}", err);
            }
        }
        eprintln!(
            "Progress: {}/{} translated={} skipped={} failed={}",
            completed,
            prepared.summary.queued,
            prepared.summary.translated,
            count_skipped(&prepared.summary),
            prepared.summary.failed
        );
    }

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                prepared.summary.failed += 1;
                eprintln!("✖ Translation worker failed: {}", err);
            }
            Err(err) => {
                prepared.summary.failed += 1;
                eprintln!("✖ Translation task failed to join: {}", err);
            }
        }
    }

    print_summary(&prepared.summary);

    if prepared.summary.failed > 0 {
        return Err("Translation failed; no files were written".to_string());
    }

    apply_translation_results(&mut prepared, &results)?;
    validate_translated_output(&prepared)?;

    if prepared.opts.dry_run {
        println!("Dry-run mode: no files were written");
    } else {
        write_back(
            &prepared.target_codec,
            &prepared.output_path,
            &prepared.output_format,
            &prepared.opts.target_lang,
        )?;
        println!("✅ Translate complete: {}", prepared.output_path);
    }

    Ok(TranslateOutcome {
        translated: prepared.summary.translated,
        skipped: count_skipped(&prepared.summary),
        failed: 0,
        output_path: Some(prepared.output_path),
    })
}

fn prepare_translation(opts: &TranslateOptions) -> Result<PreparedTranslation, String> {
    let config = load_config(opts.config.as_deref())?;
    let mut resolved = resolve_options(opts, config.as_ref())?;

    validate_path_inputs(&resolved)?;

    let source_path = resolved.source.clone();
    let target_path = resolved
        .target
        .clone()
        .unwrap_or_else(|| resolved.source.clone());
    let output_path = resolved
        .output
        .clone()
        .unwrap_or_else(|| target_path.clone());

    let output_format = infer_format_from_extension(&output_path)
        .ok_or_else(|| format!("Cannot infer output format from path: {}", output_path))?;
    let output_lang_hint = infer_language_from_path(&output_path, &output_format)
        .ok()
        .flatten();

    if opts.target.is_none()
        && output_path == source_path
        && !is_multi_language_format(&output_format)
    {
        return Err(
            "Omitting --target is only supported for in-place multi-language files; use --target or --output for single-language formats"
                .to_string(),
        );
    }

    let source_codec = read_codec(&source_path, resolved.source_lang.clone(), resolved.strict)?;
    let source_resource = select_source_resource(&source_codec, &resolved.source_lang)?;

    let mut target_codec = if Path::new(&target_path).exists() {
        read_codec(&target_path, output_lang_hint.clone(), resolved.strict)?
    } else {
        Codec::new()
    };

    if !Path::new(&target_path).exists() && is_multi_language_format(&output_format) {
        ensure_resource_exists(
            &mut target_codec,
            &source_resource.resource,
            &source_resource.language,
            true,
        );
    }

    let target_language = resolve_target_language(
        &target_codec,
        &resolved.target_lang,
        output_lang_hint.as_deref(),
    )?;
    if lang_matches(&source_resource.language, &target_language) {
        return Err("Source language and target language must differ".to_string());
    }
    resolved.target_lang = target_language;

    ensure_target_resource(&mut target_codec, &resolved.target_lang)?;
    propagate_xcstrings_metadata(&mut target_codec, &source_resource.language);

    let (jobs, summary) = build_jobs(
        &source_resource.resource,
        &target_codec,
        &resolved.target_lang,
        &resolved.statuses,
        target_supports_explicit_status(&target_path),
    )?;

    Ok(PreparedTranslation {
        opts: resolved,
        source_path,
        target_path,
        output_path,
        output_format,
        config_path: config.map(|cfg| cfg.path),
        source_resource,
        target_codec,
        jobs,
        summary,
    })
}

fn print_preamble(prepared: &PreparedTranslation) {
    println!(
        "Translating {} -> {} using {}:{}",
        prepared.source_resource.language,
        prepared.opts.target_lang,
        prepared.opts.provider.display_name(),
        prepared.opts.model
    );
    println!("Source: {}", prepared.source_path);
    println!("Target: {}", prepared.target_path);
    if let Some(config_path) = &prepared.config_path {
        println!("Config: {}", config_path.display());
    }
    if prepared.opts.dry_run {
        println!("Mode: dry-run");
    }
}

fn print_summary(summary: &TranslationSummary) {
    println!("Total source entries: {}", summary.total_entries);
    println!("Queued for translation: {}", summary.queued);
    println!("Translated: {}", summary.translated);
    println!("Skipped (plural): {}", summary.skipped_plural);
    println!(
        "Skipped (do_not_translate): {}",
        summary.skipped_do_not_translate
    );
    println!("Skipped (status): {}", summary.skipped_status);
    println!("Skipped (empty source): {}", summary.skipped_empty_source);
    println!("Failed: {}", summary.failed);
}

fn count_skipped(summary: &TranslationSummary) -> usize {
    summary.skipped_plural
        + summary.skipped_do_not_translate
        + summary.skipped_status
        + summary.skipped_empty_source
}

fn apply_translation_results(
    prepared: &mut PreparedTranslation,
    results: &HashMap<String, String>,
) -> Result<(), String> {
    for job in &prepared.jobs {
        let Some(translated_value) = results.get(&job.key) else {
            continue;
        };

        if let Some(existing) = prepared
            .target_codec
            .find_entry_mut(&job.key, &prepared.opts.target_lang)
        {
            existing.value = Translation::Singular(translated_value.clone());
            existing.status = EntryStatus::NeedsReview;
        } else {
            prepared
                .target_codec
                .add_entry(
                    &job.key,
                    &prepared.opts.target_lang,
                    Translation::Singular(translated_value.clone()),
                    job.existing_comment
                        .clone()
                        .or_else(|| job.source_comment.clone()),
                    Some(EntryStatus::NeedsReview),
                )
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn validate_translated_output(prepared: &PreparedTranslation) -> Result<(), String> {
    let mut validation_codec = prepared.target_codec.clone();
    ensure_resource_exists(
        &mut validation_codec,
        &prepared.source_resource.resource,
        &prepared.source_resource.language,
        false,
    );
    validation_codec
        .validate_placeholders(prepared.opts.strict)
        .map_err(|e| format!("Placeholder validation failed after translation: {}", e))
}

fn build_jobs(
    source: &Resource,
    target_codec: &Codec,
    target_lang: &str,
    statuses: &[EntryStatus],
    explicit_target_status: bool,
) -> Result<(Vec<TranslationJob>, TranslationSummary), String> {
    let mut jobs = Vec::new();
    let mut summary = TranslationSummary {
        total_entries: source.entries.len(),
        ..TranslationSummary::default()
    };

    for entry in &source.entries {
        match &entry.value {
            Translation::Plural(_) => {
                summary.skipped_plural += 1;
                continue;
            }
            Translation::Empty => {
                summary.skipped_empty_source += 1;
                continue;
            }
            Translation::Singular(text) if text.trim().is_empty() => {
                summary.skipped_empty_source += 1;
                continue;
            }
            Translation::Singular(text) => {
                let target_entry = target_codec.find_entry(&entry.id, target_lang);

                if entry.status == EntryStatus::DoNotTranslate
                    || target_entry.is_some_and(|item| item.status == EntryStatus::DoNotTranslate)
                {
                    summary.skipped_do_not_translate += 1;
                    continue;
                }

                let effective_status = target_entry
                    .map(|item| effective_target_status(item, explicit_target_status))
                    .unwrap_or(EntryStatus::New);

                if !statuses.contains(&effective_status) {
                    summary.skipped_status += 1;
                    continue;
                }

                jobs.push(TranslationJob {
                    key: entry.id.clone(),
                    source_lang: source.metadata.language.clone(),
                    target_lang: target_lang.to_string(),
                    source_value: text.clone(),
                    source_comment: entry.comment.clone(),
                    existing_comment: target_entry.and_then(|item| item.comment.clone()),
                });
                summary.queued += 1;
            }
        }
    }

    Ok((jobs, summary))
}

fn effective_target_status(entry: &Entry, explicit_target_status: bool) -> EntryStatus {
    if explicit_target_status {
        return entry.status.clone();
    }

    match &entry.value {
        Translation::Empty => EntryStatus::New,
        Translation::Singular(text) if text.trim().is_empty() => EntryStatus::New,
        _ => EntryStatus::Translated,
    }
}

fn ensure_target_resource(codec: &mut Codec, language: &str) -> Result<(), String> {
    if codec.get_by_language(language).is_none() {
        codec.add_resource(Resource {
            metadata: Metadata {
                language: language.to_string(),
                domain: String::new(),
                custom: HashMap::new(),
            },
            entries: Vec::new(),
        });
    }
    Ok(())
}

fn ensure_resource_exists(codec: &mut Codec, resource: &Resource, language: &str, clone_entries: bool) {
    if codec.get_by_language(language).is_some() {
        return;
    }

    codec.add_resource(Resource {
        metadata: resource.metadata.clone(),
        entries: if clone_entries {
            resource.entries.clone()
        } else {
            Vec::new()
        },
    });
}

fn propagate_xcstrings_metadata(codec: &mut Codec, source_language: &str) {
    for resource in &mut codec.resources {
        resource
            .metadata
            .custom
            .entry("source_language".to_string())
            .or_insert_with(|| source_language.to_string());
        resource
            .metadata
            .custom
            .entry("version".to_string())
            .or_insert_with(|| "1.0".to_string());
    }
}

fn validate_path_inputs(opts: &ResolvedOptions) -> Result<(), String> {
    if !Path::new(&opts.source).is_file() {
        return Err(format!("Source file does not exist: {}", opts.source));
    }

    if let Some(target) = &opts.target {
        if Path::new(target).exists() && !Path::new(target).is_file() {
            return Err(format!("Target path is not a file: {}", target));
        }
        validate_output_path(target)?;
    }

    if let Some(output) = &opts.output {
        validate_output_path(output)?;
    }

    Ok(())
}

fn resolve_options(opts: &TranslateOptions, config: Option<&LoadedConfig>) -> Result<ResolvedOptions, String> {
    let cfg = config.map(|item| &item.data.translate);
    let source_lang = opts
        .source_lang
        .clone()
        .or_else(|| cfg.and_then(|item| item.source_lang.clone()));
    let target_lang = opts
        .target_lang
        .clone()
        .or_else(|| cfg.and_then(|item| item.target_lang.clone()))
        .ok_or_else(|| "--target-lang is required (or set translate.target_lang in langcodec.toml)".to_string())?;

    validate_language_code(&target_lang)?;
    if let Some(lang) = &source_lang {
        validate_language_code(lang)?;
    }

    let provider = resolve_provider(
        opts.provider.as_deref(),
        cfg.and_then(|item| item.provider.as_deref()),
    )?;
    let model = opts
        .model
        .clone()
        .or_else(|| cfg.and_then(|item| item.model.clone()))
        .or_else(|| std::env::var("MENTRA_MODEL").ok())
        .ok_or_else(|| {
            "--model is required (or set translate.model in langcodec.toml or MENTRA_MODEL)"
                .to_string()
        })?;

    let concurrency = opts
        .concurrency
        .or_else(|| cfg.and_then(|item| item.concurrency))
        .unwrap_or(DEFAULT_CONCURRENCY);
    if concurrency == 0 {
        return Err("Concurrency must be greater than zero".to_string());
    }

    let statuses = parse_status_filter(
        opts.status.as_deref(),
        cfg.and_then(|item| item.status.as_ref()),
    )?;

    Ok(ResolvedOptions {
        source: opts.source.clone(),
        target: opts.target.clone(),
        output: opts.output.clone(),
        source_lang,
        target_lang,
        statuses,
        provider,
        model,
        concurrency,
        dry_run: opts.dry_run,
        strict: opts.strict,
    })
}

fn resolve_provider(cli: Option<&str>, cfg: Option<&str>) -> Result<ProviderKind, String> {
    if let Some(value) = cli {
        return ProviderKind::parse(value);
    }
    if let Some(value) = cfg {
        return ProviderKind::parse(value);
    }

    let mut available = Vec::new();
    for kind in [
        ProviderKind::OpenAI,
        ProviderKind::Anthropic,
        ProviderKind::Gemini,
    ] {
        if std::env::var(kind.api_key_env()).is_ok() {
            available.push(kind);
        }
    }

    match available.len() {
        1 => Ok(available.remove(0)),
        0 => Err(
            "--provider is required (or set translate.provider in langcodec.toml or configure exactly one provider API key)"
                .to_string(),
        ),
        _ => Err(
            "Multiple provider API keys are configured; specify --provider or translate.provider in langcodec.toml"
                .to_string(),
        ),
    }
}

fn parse_status_filter(cli: Option<&str>, cfg: Option<&Vec<String>>) -> Result<Vec<EntryStatus>, String> {
    let raw_values: Vec<String> = if let Some(cli) = cli {
        cli.split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    } else if let Some(cfg) = cfg {
        cfg.clone()
    } else {
        DEFAULT_STATUSES.iter().map(|value| value.to_string()).collect()
    };

    let mut statuses = Vec::new();
    for raw in raw_values {
        let normalized = raw.replace(['-', ' '], "_");
        let parsed = normalized
            .parse::<EntryStatus>()
            .map_err(|e| format!("Invalid translate status '{}': {}", raw, e))?;
        if !statuses.contains(&parsed) {
            statuses.push(parsed);
        }
    }
    Ok(statuses)
}

fn read_codec(path: &str, language_hint: Option<String>, strict: bool) -> Result<Codec, String> {
    let mut codec = Codec::new();
    codec.read_file_by_extension_with_options(
        path,
        &ReadOptions::new()
            .with_language_hint(language_hint)
            .with_strict(strict),
    )
    .map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    Ok(codec)
}

fn select_source_resource(codec: &Codec, requested_lang: &Option<String>) -> Result<SelectedResource, String> {
    if let Some(lang) = requested_lang {
        let resource = codec
            .resources
            .iter()
            .find(|item| lang_matches(&item.metadata.language, lang))
            .cloned()
            .ok_or_else(|| format!("Source language '{}' not found", lang))?;
        return Ok(SelectedResource {
            language: resource.metadata.language.clone(),
            resource,
        });
    }

    if codec.resources.len() == 1 {
        let resource = codec.resources[0].clone();
        return Ok(SelectedResource {
            language: resource.metadata.language.clone(),
            resource,
        });
    }

    Err("Multiple source languages present; specify --source-lang".to_string())
}

fn resolve_target_language(
    codec: &Codec,
    requested_lang: &str,
    inferred_from_output: Option<&str>,
) -> Result<String, String> {
    if let Some(resource) = codec
        .resources
        .iter()
        .find(|item| lang_matches(&item.metadata.language, requested_lang))
    {
        return Ok(resource.metadata.language.clone());
    }

    if let Some(inferred) = inferred_from_output
        && lang_matches(inferred, requested_lang)
    {
        return Ok(inferred.to_string());
    }

    Ok(requested_lang.to_string())
}

fn lang_matches(resource_lang: &str, requested_lang: &str) -> bool {
    normalize_lang(resource_lang) == normalize_lang(requested_lang)
        || normalize_lang(resource_lang)
            .split('-')
            .next()
            .unwrap_or(resource_lang)
            == normalize_lang(requested_lang)
                .split('-')
                .next()
                .unwrap_or(requested_lang)
}

fn normalize_lang(lang: &str) -> String {
    lang.trim().replace('_', "-").to_ascii_lowercase()
}

fn is_multi_language_format(format: &FormatType) -> bool {
    matches!(format, FormatType::Xcstrings | FormatType::CSV | FormatType::TSV)
}

fn target_supports_explicit_status(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xcstrings"))
}

fn write_back(
    codec: &Codec,
    output_path: &str,
    output_format: &FormatType,
    target_lang: &str,
) -> Result<(), String> {
    match output_format {
        FormatType::Strings(_) | FormatType::AndroidStrings(_) => {
            let resource = codec
                .resources
                .iter()
                .find(|item| lang_matches(&item.metadata.language, target_lang))
                .ok_or_else(|| format!("Target language '{}' not found in output", target_lang))?;
            Codec::write_resource_to_file(resource, output_path)
                .map_err(|e| format!("Error writing output: {}", e))
        }
        FormatType::Xcstrings | FormatType::CSV | FormatType::TSV => {
            convert_resources_to_format(codec.resources.clone(), output_path, output_format.clone())
                .map_err(|e| format!("Error writing output: {}", e))
        }
    }
}

fn create_mentra_backend(opts: &ResolvedOptions) -> Result<MentraBackend, String> {
    let setup = build_provider(&opts.provider)?;
    if setup.provider_kind != opts.provider {
        return Err("Resolved provider mismatch".to_string());
    }
    Ok(MentraBackend {
        provider: setup.provider,
        model: opts.model.clone(),
    })
}

fn build_provider(kind: &ProviderKind) -> Result<ProviderSetup, String> {
    let api_key = std::env::var(kind.api_key_env()).map_err(|_| {
        format!(
            "Missing {} environment variable for {} provider",
            kind.api_key_env(),
            kind.display_name()
        )
    })?;

    let provider: Arc<dyn Provider> = match kind {
        ProviderKind::OpenAI => Arc::new(provider::openai::OpenAIProvider::new(api_key)),
        ProviderKind::Anthropic => Arc::new(provider::anthropic::AnthropicProvider::new(api_key)),
        ProviderKind::Gemini => Arc::new(provider::gemini::GeminiProvider::new(api_key)),
    };

    Ok(ProviderSetup {
        provider_kind: kind.clone(),
        provider,
    })
}

fn build_prompt(request: &BackendRequest) -> String {
    let mut prompt = format!(
        "Translate the following localization value from {} to {}.\nKey: {}\nSource value:\n{}\n",
        request.source_lang, request.target_lang, request.key, request.source_value
    );
    if let Some(comment) = &request.source_comment {
        prompt.push_str("\nComment:\n");
        prompt.push_str(comment);
        prompt.push('\n');
    }
    prompt.push_str(
        "\nReturn JSON only in this exact shape: {\"translation\":\"...\"}. Do not wrap in markdown fences unless necessary.",
    );
    prompt
}

fn collect_text_blocks(response: &provider::Response) -> String {
    response
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn parse_translation_response(text: &str) -> Result<String, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Model returned an empty translation".to_string());
    }

    if let Ok(payload) = serde_json::from_str::<ModelTranslationPayload>(trimmed) {
        return Ok(payload.translation);
    }

    if let Some(json_body) = extract_json_body(trimmed)
        && let Ok(payload) = serde_json::from_str::<ModelTranslationPayload>(&json_body)
    {
        return Ok(payload.translation);
    }

    Err(format!(
        "Model response was not valid translation JSON: {}",
        trimmed
    ))
}

fn extract_json_body(text: &str) -> Option<String> {
    let fenced = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .map(str::trim_start)?;
    let unfenced = fenced.strip_suffix("```")?.trim();
    Some(unfenced.to_string())
}

fn format_provider_error(err: ProviderError) -> String {
    format!("Provider request failed: {}", err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::Mutex};
    use tempfile::TempDir;

    #[derive(Clone)]
    struct MockBackend {
        responses: Arc<Mutex<HashMap<String, Result<String, String>>>>,
    }

    impl MockBackend {
        fn new(responses: Vec<(&str, Result<String, String>)>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(
                    responses
                        .into_iter()
                        .map(|(key, value)| (key.to_string(), value))
                        .collect(),
                )),
            }
        }
    }

    #[async_trait]
    impl TranslationBackend for MockBackend {
        async fn translate(&self, request: BackendRequest) -> Result<String, String> {
            self.responses
                .lock()
                .unwrap()
                .remove(&request.key)
                .unwrap_or_else(|| Err("missing mock response".to_string()))
        }
    }

    fn base_options(source: &Path, target: Option<&Path>) -> TranslateOptions {
        TranslateOptions {
            source: source.to_string_lossy().to_string(),
            target: target.map(|path| path.to_string_lossy().to_string()),
            output: None,
            source_lang: Some("en".to_string()),
            target_lang: Some("fr".to_string()),
            status: None,
            provider: Some("openai".to_string()),
            model: Some("gpt-4.1-mini".to_string()),
            concurrency: Some(2),
            config: None,
            dry_run: false,
            strict: false,
        }
    }

    #[test]
    fn translates_missing_entries_into_target_file() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("en.strings");
        let target = temp_dir.path().join("fr.strings");

        fs::write(&source, "\"welcome\" = \"Welcome\";\n\"bye\" = \"Goodbye\";\n").unwrap();

        let prepared = prepare_translation(&base_options(&source, Some(&target))).unwrap();
        let outcome = run_prepared_translation(
            prepared,
            Arc::new(MockBackend::new(vec![
                ("welcome", Ok("Bienvenue".to_string())),
                ("bye", Ok("Au revoir".to_string())),
            ])),
        )
        .unwrap();

        assert_eq!(outcome.translated, 2);
        let written = fs::read_to_string(&target).unwrap();
        assert!(written.contains("\"welcome\" = \"Bienvenue\";"));
        assert!(written.contains("\"bye\" = \"Au revoir\";"));
    }

    #[test]
    fn dry_run_does_not_write_target() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("en.strings");
        let target = temp_dir.path().join("fr.strings");

        fs::write(&source, "\"welcome\" = \"Welcome\";\n").unwrap();
        fs::write(&target, "\"welcome\" = \"\";\n").unwrap();

        let mut options = base_options(&source, Some(&target));
        options.dry_run = true;

        let before = fs::read_to_string(&target).unwrap();
        let prepared = prepare_translation(&options).unwrap();
        let outcome = run_prepared_translation(
            prepared,
            Arc::new(MockBackend::new(vec![(
                "welcome",
                Ok("Bienvenue".to_string()),
            )])),
        )
        .unwrap();
        let after = fs::read_to_string(&target).unwrap();

        assert_eq!(outcome.translated, 1);
        assert_eq!(before, after);
    }

    #[test]
    fn fails_without_writing_when_any_translation_fails() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("en.strings");
        let target = temp_dir.path().join("fr.strings");

        fs::write(&source, "\"welcome\" = \"Welcome\";\n\"bye\" = \"Goodbye\";\n").unwrap();
        fs::write(&target, "\"welcome\" = \"\";\n\"bye\" = \"\";\n").unwrap();
        let before = fs::read_to_string(&target).unwrap();

        let prepared = prepare_translation(&base_options(&source, Some(&target))).unwrap();
        let err = run_prepared_translation(
            prepared,
            Arc::new(MockBackend::new(vec![
                ("welcome", Ok("Bienvenue".to_string())),
                ("bye", Err("boom".to_string())),
            ])),
        )
        .unwrap_err();

        assert!(err.contains("no files were written"));
        let after = fs::read_to_string(&target).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn uses_config_defaults_when_flags_are_missing() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.csv");
        let config = temp_dir.path().join("langcodec.toml");
        fs::write(&source, "key,en,fr\nwelcome,Welcome,\n").unwrap();
        fs::write(
            &config,
            r#"[translate]
provider = "openai"
model = "gpt-4.1-mini"
source_lang = "en"
target_lang = "fr"
concurrency = 2
status = ["new", "stale"]
"#,
        )
        .unwrap();

        let options = TranslateOptions {
            source: source.to_string_lossy().to_string(),
            target: None,
            output: None,
            source_lang: None,
            target_lang: None,
            status: None,
            provider: None,
            model: None,
            concurrency: None,
            config: Some(config.to_string_lossy().to_string()),
            dry_run: true,
            strict: false,
        };

        let prepared = prepare_translation(&options).unwrap();
        assert_eq!(prepared.opts.model, "gpt-4.1-mini");
        assert_eq!(prepared.opts.target_lang, "fr");
        assert_eq!(prepared.summary.queued, 1);
    }

    #[test]
    fn skips_plural_entries() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("Localizable.xcstrings");
        let target = temp_dir.path().join("translated.xcstrings");
        fs::write(
            &source,
            r#"{
  "sourceLanguage" : "en",
  "version" : "1.0",
  "strings" : {
    "welcome" : {
      "localizations" : {
        "en" : {
          "stringUnit" : {
            "state" : "new",
            "value" : "Welcome"
          }
        }
      }
    },
    "item_count" : {
      "localizations" : {
        "en" : {
          "variations" : {
            "plural" : {
              "one" : {
                "stringUnit" : {
                  "state" : "new",
                  "value" : "%#@items@"
                }
              },
              "other" : {
                "stringUnit" : {
                  "state" : "new",
                  "value" : "%#@items@"
                }
              }
            }
          }
        }
      }
    }
  }
}"#,
        )
        .unwrap();

        let prepared = prepare_translation(&base_options(&source, Some(&target))).unwrap();
        assert_eq!(prepared.summary.skipped_plural, 1);
        assert_eq!(prepared.summary.queued, 1);
    }

    #[test]
    fn rejects_in_place_single_language_translation_without_target() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("en.strings");
        fs::write(&source, "\"welcome\" = \"Welcome\";\n").unwrap();

        let options = base_options(&source, None);
        let err = prepare_translation(&options).unwrap_err();
        assert!(err.contains("Omitting --target is only supported"));
    }

    #[test]
    fn canonicalizes_target_language_from_existing_target_resource() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("translations.csv");
        let target = temp_dir.path().join("target.csv");
        fs::write(&source, "key,en\nwelcome,Welcome\n").unwrap();
        fs::write(&target, "key,fr-CA\nwelcome,\n").unwrap();

        let mut options = base_options(&source, Some(&target));
        options.target_lang = Some("fr".to_string());
        options.source_lang = Some("en".to_string());

        let prepared = prepare_translation(&options).unwrap();
        assert_eq!(prepared.opts.target_lang, "fr-CA");
        assert_eq!(prepared.summary.queued, 1);
    }

    #[test]
    fn infers_status_from_target_input_format_not_output_format() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("en.strings");
        let target = temp_dir.path().join("fr.strings");
        let output = temp_dir.path().join("translated.xcstrings");

        fs::write(&source, "\"welcome\" = \"Welcome\";\n").unwrap();
        fs::write(&target, "\"welcome\" = \"\";\n").unwrap();

        let mut options = base_options(&source, Some(&target));
        options.output = Some(output.to_string_lossy().to_string());

        let prepared = prepare_translation(&options).unwrap();
        assert_eq!(prepared.summary.queued, 1);
    }

    #[test]
    fn parses_fenced_json_translation() {
        let text = "```json\n{\"translation\":\"Bonjour\"}\n```";
        let parsed = parse_translation_response(text).unwrap();
        assert_eq!(parsed, "Bonjour");
    }
}
