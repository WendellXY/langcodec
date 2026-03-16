use crate::{
    ai::{ProviderKind, read_api_key, resolve_model, resolve_provider},
    config::{LoadedConfig, load_config, resolve_config_relative_path},
    validation::validate_language_code,
};
use async_trait::async_trait;
use langcodec::{
    Resource, Translation,
    formats::{XcstringsFormat, xcstrings::Item},
    traits::Parser,
};
use mentra::{
    AgentConfig, ContentBlock, ModelInfo, Runtime,
    agent::{AgentEvent, ToolProfile, WorkspaceConfig},
    provider::ProviderRequestOptions,
    runtime::RunOptions,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    runtime::Builder,
    sync::{Mutex as AsyncMutex, broadcast, mpsc},
    task::JoinSet,
};

const DEFAULT_CONCURRENCY: usize = 4;
const DEFAULT_TOOL_BUDGET: usize = 16;
const ANNOTATION_SYSTEM_PROMPT: &str = "You write translator-facing comments for Xcode xcstrings entries. Use the files tool or shell tool when needed to inspect source code. Prefer shell commands like rg for fast code search, then read the most relevant files before drafting. Prefer a short, concrete explanation of where or how the text is used so a translator can choose the right wording. If you are uncertain, say what the UI usage appears to be instead of inventing product meaning. Return JSON only with the shape {\"comment\":\"...\",\"confidence\":\"high|medium|low\"}.";

#[derive(Debug, Clone)]
pub struct AnnotateOptions {
    pub input: Option<String>,
    pub source_roots: Vec<String>,
    pub output: Option<String>,
    pub source_lang: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub concurrency: Option<usize>,
    pub config: Option<String>,
    pub dry_run: bool,
    pub check: bool,
}

#[derive(Debug, Clone)]
struct ResolvedAnnotateOptions {
    input: String,
    output: String,
    source_roots: Vec<String>,
    source_lang: Option<String>,
    provider: ProviderKind,
    model: String,
    concurrency: usize,
    dry_run: bool,
    check: bool,
    workspace_root: PathBuf,
}

#[derive(Debug, Clone)]
struct AnnotationRequest {
    key: String,
    source_lang: String,
    source_value: String,
    existing_comment: Option<String>,
    source_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct AnnotationResponse {
    comment: String,
    confidence: String,
}

enum WorkerUpdate {
    Started {
        worker_id: usize,
        key: String,
        candidate_count: usize,
        top_candidate: Option<String>,
    },
    Finished {
        worker_id: usize,
        key: String,
        result: Result<Option<AnnotationResponse>, String>,
    },
}

struct AnnotateProgressRenderer {
    interactive: bool,
    last_width: usize,
}

impl AnnotateProgressRenderer {
    fn new() -> Self {
        Self {
            interactive: io::stderr().is_terminal(),
            last_width: 0,
        }
    }

    fn print_message(&mut self, message: &str) {
        self.finish_line();
        eprintln!("{}", message);
    }

    fn start_annotation(&mut self, total: usize, concurrency: usize) {
        self.print_message(&format!(
            "Generating translator comments for {} entr{} with {} worker(s)...",
            total,
            if total == 1 { "y" } else { "ies" },
            concurrency
        ));
    }

    fn worker_started(
        &mut self,
        worker_id: usize,
        key: &str,
        candidate_count: usize,
        top_candidate: Option<&str>,
    ) {
        let mut message = format!(
            "Worker {} started key={} shortlist={}",
            worker_id, key, candidate_count
        );
        if let Some(path) = top_candidate {
            message.push_str(" top=");
            message.push_str(path);
        }
        self.print_message(&message);
    }

    fn worker_finished(
        &mut self,
        worker_id: usize,
        key: &str,
        result: &Result<Option<AnnotationResponse>, String>,
    ) {
        let status = match result {
            Ok(Some(_)) => "generated",
            Ok(None) => "skipped",
            Err(_) => "failed",
        };
        self.print_message(&format!(
            "Worker {} finished key={} result={}",
            worker_id, key, status
        ));
    }

    fn update_annotation(
        &mut self,
        completed: usize,
        total: usize,
        generated: usize,
        unmatched: usize,
    ) {
        self.render_line(&format!(
            "Annotate progress: {}/{} processed generated={} skipped={}",
            completed, total, generated, unmatched
        ));
    }

    fn render_line(&mut self, line: &str) {
        if self.interactive {
            let padding = self.last_width.saturating_sub(line.len());
            eprint!("\r{}{}", line, " ".repeat(padding));
            let _ = io::stderr().flush();
            self.last_width = line.len();
        } else {
            eprintln!("{}", line);
        }
    }

    fn finish_line(&mut self) {
        if self.interactive && self.last_width > 0 {
            eprintln!();
            self.last_width = 0;
        }
    }
}

#[async_trait]
trait AnnotationBackend: Send + Sync {
    async fn annotate(
        &self,
        request: AnnotationRequest,
    ) -> Result<Option<AnnotationResponse>, String>;
}

struct MentraAnnotatorBackend {
    runtime: Arc<Runtime>,
    model: ModelInfo,
    workspace_root: PathBuf,
}

impl MentraAnnotatorBackend {
    fn new(opts: &ResolvedAnnotateOptions) -> Result<Self, String> {
        let api_key = read_api_key(&opts.provider)?;
        let provider = opts.provider.builtin_provider();
        let runtime = Runtime::builder()
            .with_provider(provider, api_key)
            .build()
            .map_err(|e| format!("Failed to build Mentra runtime: {}", e))?;

        Ok(Self {
            runtime: Arc::new(runtime),
            model: ModelInfo::new(opts.model.clone(), provider),
            workspace_root: opts.workspace_root.clone(),
        })
    }

    #[cfg(test)]
    fn from_runtime(runtime: Runtime, model: ModelInfo, workspace_root: PathBuf) -> Self {
        Self {
            runtime: Arc::new(runtime),
            model,
            workspace_root,
        }
    }
}

#[async_trait]
impl AnnotationBackend for MentraAnnotatorBackend {
    async fn annotate(
        &self,
        request: AnnotationRequest,
    ) -> Result<Option<AnnotationResponse>, String> {
        let config = build_agent_config(&self.workspace_root);
        let mut agent = self
            .runtime
            .spawn_with_config("annotate", self.model.clone(), config)
            .map_err(|e| format!("Failed to spawn Mentra agent: {}", e))?;
        let tool_logger = spawn_tool_call_logger(agent.subscribe_events(), request.key.clone());

        let response = agent
            .run(
                vec![ContentBlock::text(build_annotation_prompt(&request))],
                RunOptions {
                    tool_budget: Some(DEFAULT_TOOL_BUDGET),
                    ..RunOptions::default()
                },
            )
            .await;
        tool_logger.abort();
        let _ = tool_logger.await;

        let response = response.map_err(|e| format!("Annotation agent failed: {}", e))?;

        parse_annotation_response(&response.text()).map(Some)
    }
}

pub fn run_annotate_command(opts: AnnotateOptions) -> Result<(), String> {
    let config = load_config(opts.config.as_deref())?;
    let runs = expand_annotate_invocations(&opts, config.as_ref())?;

    for resolved in runs {
        let backend: Arc<dyn AnnotationBackend> = Arc::new(MentraAnnotatorBackend::new(&resolved)?);
        run_annotate_with_backend(resolved, backend)?;
    }

    Ok(())
}

fn run_annotate_with_backend(
    opts: ResolvedAnnotateOptions,
    backend: Arc<dyn AnnotationBackend>,
) -> Result<(), String> {
    let mut progress = AnnotateProgressRenderer::new();
    progress.print_message(&format!("Annotating {}", opts.input));
    let mut catalog = XcstringsFormat::read_from(&opts.input)
        .map_err(|e| format!("Failed to read '{}': {}", opts.input, e))?;
    let resources = Vec::<Resource>::try_from(catalog.clone())
        .map_err(|e| format!("Failed to decode xcstrings '{}': {}", opts.input, e))?;

    let source_lang = opts
        .source_lang
        .clone()
        .unwrap_or_else(|| catalog.source_language.clone());
    validate_language_code(&source_lang)?;

    let source_values = source_value_map(&resources, &source_lang);
    let requests = build_annotation_requests(
        &catalog,
        &source_lang,
        &source_values,
        &opts.source_roots,
        &opts.workspace_root,
        &mut progress,
    )?;

    if requests.is_empty() {
        progress.finish_line();
        println!("No entries require annotation updates.");
        return Ok(());
    }

    progress.start_annotation(requests.len(), opts.concurrency);
    let results = annotate_requests(requests, backend, opts.concurrency, &mut progress);
    progress.finish_line();
    let results = results?;
    let mut changed = 0usize;
    let mut unmatched = 0usize;

    let mut keys = catalog.strings.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    for key in keys {
        let Some(item) = catalog.strings.get_mut(&key) else {
            continue;
        };
        if should_preserve_manual_comment(item) {
            continue;
        }

        match results.get(&key) {
            Some(Some(annotation)) => {
                if item.comment.as_deref() != Some(annotation.comment.as_str())
                    || item.is_comment_auto_generated != Some(true)
                {
                    item.comment = Some(annotation.comment.clone());
                    item.is_comment_auto_generated = Some(true);
                    changed += 1;
                }
            }
            Some(None) => unmatched += 1,
            None => {}
        }
    }

    if opts.check && changed > 0 {
        println!("would change: {}", opts.output);
        return Err(format!("would change: {}", opts.output));
    }

    if opts.dry_run {
        println!(
            "DRY-RUN: would update {} comment(s) in {}",
            changed, opts.output
        );
        if unmatched > 0 {
            println!("Skipped {} entry(s) without generated comments", unmatched);
        }
        return Ok(());
    }

    if changed == 0 {
        println!("No comment updates were necessary.");
        if unmatched > 0 {
            println!("Skipped {} entry(s) without generated comments", unmatched);
        }
        return Ok(());
    }

    catalog
        .write_to(&opts.output)
        .map_err(|e| format!("Failed to write '{}': {}", opts.output, e))?;

    println!("Updated {} comment(s) in {}", changed, opts.output);
    if unmatched > 0 {
        println!("Skipped {} entry(s) without generated comments", unmatched);
    }
    Ok(())
}

fn expand_annotate_invocations(
    opts: &AnnotateOptions,
    config: Option<&LoadedConfig>,
) -> Result<Vec<ResolvedAnnotateOptions>, String> {
    let cfg = config.map(|item| &item.data.annotate);
    let config_dir = config.and_then(LoadedConfig::config_dir);

    if cfg
        .and_then(|item| item.input.as_ref())
        .is_some_and(|_| cfg.and_then(|item| item.inputs.as_ref()).is_some())
    {
        return Err("Config annotate.input and annotate.inputs cannot both be set".to_string());
    }

    let inputs = resolve_config_inputs(opts, cfg, config_dir)?;
    if inputs.is_empty() {
        return Err(
            "--input is required unless annotate.input or annotate.inputs is set in langcodec.toml"
                .to_string(),
        );
    }

    let output = if let Some(output) = &opts.output {
        Some(output.clone())
    } else {
        cfg.and_then(|item| item.output.clone())
            .map(|path| resolve_config_relative_path(config_dir, &path))
    };

    if inputs.len() > 1 && output.is_some() {
        return Err(
            "annotate.inputs cannot be combined with annotate.output or CLI --output; use in-place annotation for multiple inputs"
                .to_string(),
        );
    }

    inputs
        .into_iter()
        .map(|input| {
            resolve_annotate_options(
                &AnnotateOptions {
                    input: Some(input),
                    source_roots: opts.source_roots.clone(),
                    output: output.clone(),
                    source_lang: opts.source_lang.clone(),
                    provider: opts.provider.clone(),
                    model: opts.model.clone(),
                    concurrency: opts.concurrency,
                    config: opts.config.clone(),
                    dry_run: opts.dry_run,
                    check: opts.check,
                },
                config,
            )
        })
        .collect()
}

fn resolve_config_inputs(
    opts: &AnnotateOptions,
    cfg: Option<&crate::config::AnnotateConfig>,
    config_dir: Option<&Path>,
) -> Result<Vec<String>, String> {
    if let Some(input) = &opts.input {
        return Ok(vec![input.clone()]);
    }

    if let Some(input) = cfg.and_then(|item| item.input.as_ref()) {
        return Ok(vec![resolve_config_relative_path(config_dir, input)]);
    }

    if let Some(inputs) = cfg.and_then(|item| item.inputs.as_ref()) {
        return Ok(inputs
            .iter()
            .map(|input| resolve_config_relative_path(config_dir, input))
            .collect());
    }

    Ok(Vec::new())
}

fn resolve_annotate_options(
    opts: &AnnotateOptions,
    config: Option<&LoadedConfig>,
) -> Result<ResolvedAnnotateOptions, String> {
    let cfg = config.map(|item| &item.data.annotate);
    let config_dir = config.and_then(LoadedConfig::config_dir);
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Failed to determine current directory: {}", e))?;

    let input = if let Some(input) = &opts.input {
        absolutize_path(input, &cwd)
    } else if let Some(input) = cfg.and_then(|item| item.input.as_deref()) {
        absolutize_path(&resolve_config_relative_path(config_dir, input), &cwd)
    } else {
        return Err(
            "--input is required unless annotate.input or annotate.inputs is set in langcodec.toml"
                .to_string(),
        );
    };

    let source_roots = if !opts.source_roots.is_empty() {
        opts.source_roots
            .iter()
            .map(|path| absolutize_path(path, &cwd))
            .collect::<Vec<_>>()
    } else if let Some(roots) = cfg.and_then(|item| item.source_roots.as_ref()) {
        roots
            .iter()
            .map(|path| absolutize_path(&resolve_config_relative_path(config_dir, path), &cwd))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if source_roots.is_empty() {
        return Err(
            "--source-root is required unless annotate.source_roots is set in langcodec.toml"
                .to_string(),
        );
    }
    for root in &source_roots {
        let path = Path::new(root);
        if !path.is_dir() {
            return Err(format!(
                "Source root does not exist or is not a directory: {}",
                root
            ));
        }
    }

    let output = if let Some(output) = &opts.output {
        absolutize_path(output, &cwd)
    } else if let Some(output) = cfg.and_then(|item| item.output.as_deref()) {
        absolutize_path(&resolve_config_relative_path(config_dir, output), &cwd)
    } else {
        input.clone()
    };

    let concurrency = opts
        .concurrency
        .or_else(|| cfg.and_then(|item| item.concurrency))
        .unwrap_or(DEFAULT_CONCURRENCY);
    if concurrency == 0 {
        return Err("Concurrency must be greater than zero".to_string());
    }

    let provider = resolve_provider(
        opts.provider.as_deref(),
        config.and_then(|item| item.data.shared_provider()),
        None,
    )?;
    let model = resolve_model(
        opts.model.as_deref(),
        config.and_then(|item| item.data.shared_model()),
        None,
    )?;

    let source_lang = opts
        .source_lang
        .clone()
        .or_else(|| cfg.and_then(|item| item.source_lang.clone()));
    if let Some(lang) = &source_lang {
        validate_language_code(lang)?;
    }

    let workspace_root = derive_workspace_root(&input, &source_roots, &cwd);

    Ok(ResolvedAnnotateOptions {
        input,
        output,
        source_roots,
        source_lang,
        provider,
        model,
        concurrency,
        dry_run: opts.dry_run,
        check: opts.check,
        workspace_root,
    })
}

fn annotate_requests(
    requests: Vec<AnnotationRequest>,
    backend: Arc<dyn AnnotationBackend>,
    concurrency: usize,
    progress: &mut AnnotateProgressRenderer,
) -> Result<BTreeMap<String, Option<AnnotationResponse>>, String> {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to start async runtime: {}", e))?;

    let total = requests.len();
    runtime.block_on(async {
        let worker_count = concurrency.min(total).max(1);
        let queue = Arc::new(AsyncMutex::new(VecDeque::from(requests)));
        let (tx, mut rx) = mpsc::unbounded_channel::<WorkerUpdate>();
        let mut set = JoinSet::new();
        for worker_id in 1..=worker_count {
            let backend = Arc::clone(&backend);
            let queue = Arc::clone(&queue);
            let tx = tx.clone();
            set.spawn(async move {
                loop {
                    let request = {
                        let mut queue = queue.lock().await;
                        queue.pop_front()
                    };

                    let Some(request) = request else {
                        break;
                    };

                    let key = request.key.clone();
                    let _ = tx.send(WorkerUpdate::Started {
                        worker_id,
                        key: key.clone(),
                        candidate_count: 0,
                        top_candidate: None,
                    });
                    let result = backend.annotate(request).await;
                    let _ = tx.send(WorkerUpdate::Finished {
                        worker_id,
                        key,
                        result,
                    });
                }

                Ok::<(), String>(())
            });
        }
        drop(tx);

        let mut results = BTreeMap::new();
        let mut completed = 0usize;
        let mut generated = 0usize;
        let mut unmatched = 0usize;
        let mut first_error = None;

        while let Some(update) = rx.recv().await {
            match update {
                WorkerUpdate::Started {
                    worker_id,
                    key,
                    candidate_count,
                    top_candidate,
                } => {
                    progress.worker_started(
                        worker_id,
                        &key,
                        candidate_count,
                        top_candidate.as_deref(),
                    );
                }
                WorkerUpdate::Finished {
                    worker_id,
                    key,
                    result,
                } => {
                    progress.worker_finished(worker_id, &key, &result);
                    completed += 1;
                    match result {
                        Ok(annotation) => {
                            if annotation.is_some() {
                                generated += 1;
                            } else {
                                unmatched += 1;
                            }
                            results.insert(key, annotation);
                        }
                        Err(err) => {
                            if first_error.is_none() {
                                first_error = Some(err);
                            }
                        }
                    }
                    progress.update_annotation(completed, total, generated, unmatched);
                }
            }
        }

        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                }
                Err(err) => {
                    if first_error.is_none() {
                        first_error = Some(format!("Annotation task failed: {}", err));
                    }
                }
            }
        }

        if let Some(err) = first_error {
            return Err(err);
        }

        Ok(results)
    })
}

fn build_annotation_requests(
    catalog: &XcstringsFormat,
    source_lang: &str,
    source_values: &HashMap<String, String>,
    source_roots: &[String],
    workspace_root: &Path,
    _progress: &mut AnnotateProgressRenderer,
) -> Result<Vec<AnnotationRequest>, String> {
    let mut keys = catalog.strings.keys().cloned().collect::<Vec<_>>();
    keys.sort();

    let mut requests = Vec::new();
    for key in keys {
        let Some(item) = catalog.strings.get(&key) else {
            continue;
        };
        if should_preserve_manual_comment(item) {
            continue;
        }

        let source_value = source_values
            .get(&key)
            .cloned()
            .unwrap_or_else(|| key.clone());

        requests.push(AnnotationRequest {
            key,
            source_lang: source_lang.to_string(),
            source_value,
            existing_comment: item.comment.clone(),
            source_roots: source_roots
                .iter()
                .map(|root| display_path(workspace_root, Path::new(root)))
                .collect(),
        });
    }

    Ok(requests)
}

fn should_preserve_manual_comment(item: &Item) -> bool {
    item.comment.is_some() && item.is_comment_auto_generated != Some(true)
}

fn source_value_map(resources: &[Resource], source_lang: &str) -> HashMap<String, String> {
    resources
        .iter()
        .find(|resource| lang_matches(&resource.metadata.language, source_lang))
        .map(|resource| {
            resource
                .entries
                .iter()
                .map(|entry| {
                    (
                        entry.id.clone(),
                        translation_to_text(&entry.value, &entry.id),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn translation_to_text(value: &Translation, fallback_key: &str) -> String {
    match value {
        Translation::Empty => fallback_key.to_string(),
        Translation::Singular(text) => text.clone(),
        Translation::Plural(plural) => plural
            .forms
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| fallback_key.to_string()),
    }
}

fn build_agent_config(workspace_root: &Path) -> AgentConfig {
    AgentConfig {
        system: Some(ANNOTATION_SYSTEM_PROMPT.to_string()),
        temperature: Some(0.2),
        max_output_tokens: Some(512),
        tool_profile: ToolProfile::only(["files", "shell"]),
        provider_request_options: ProviderRequestOptions {
            openai: mentra::provider::OpenAIRequestOptions {
                parallel_tool_calls: Some(false),
            },
            ..ProviderRequestOptions::default()
        },
        workspace: WorkspaceConfig {
            base_dir: workspace_root.to_path_buf(),
            auto_route_shell: false,
        },
        ..AgentConfig::default()
    }
}

fn build_annotation_prompt(request: &AnnotationRequest) -> String {
    let mut prompt = format!(
        "Write one translator-facing comment for this xcstrings entry.\n\nKey: {}\nSource language: {}\nSource value: {}\n",
        request.key, request.source_lang, request.source_value
    );

    if let Some(existing_comment) = &request.existing_comment {
        prompt.push_str("\nExisting auto-generated comment:\n");
        prompt.push_str(existing_comment);
        prompt.push('\n');
    }

    prompt.push_str("\nSource roots you may inspect with the files tool:\n");
    for root in &request.source_roots {
        prompt.push_str("- ");
        prompt.push_str(root);
        prompt.push('\n');
    }

    prompt.push_str(
        "\nUse the shell tool for fast code search, preferably with rg, within these roots before drafting when the usage is not already obvious. Then use files reads for only the most relevant hits. Avoid broad repeated searches or directory listings.\n",
    );

    prompt.push_str(
        "\nRequirements:\n- Keep the comment concise and useful for translators.\n- Prefer describing UI role or user-facing context.\n- If confidence is low, mention the concrete code usage you found instead of guessing product meaning.\n- Use as few tool calls as practical; usually one rg search plus a small number of targeted file reads is enough.\n- Do not mention internal file paths unless they clarify usage.\n- Return JSON only: {\"comment\":\"...\",\"confidence\":\"high|medium|low\"}.\n",
    );
    prompt
}

fn spawn_tool_call_logger(
    mut events: broadcast::Receiver<AgentEvent>,
    key: String,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match events.recv().await {
                Ok(AgentEvent::ToolExecutionStarted { call }) => {
                    eprintln!(
                        "Tool call key={} tool={} input={}",
                        key,
                        call.name,
                        compact_tool_input(&call.input)
                    );
                }
                Ok(AgentEvent::ToolExecutionFinished { result }) => {
                    let status = match result {
                        ContentBlock::ToolResult { is_error, .. } if is_error => "error",
                        ContentBlock::ToolResult { .. } => "ok",
                        _ => "unknown",
                    };
                    eprintln!("Tool result key={} status={}", key, status);
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    })
}

fn compact_tool_input(input: &Value) -> String {
    const MAX_TOOL_INPUT_CHARS: usize = 180;

    let rendered = serde_json::to_string(input).unwrap_or_else(|_| "<unserializable>".to_string());
    let mut preview = rendered
        .chars()
        .take(MAX_TOOL_INPUT_CHARS)
        .collect::<String>();
    if rendered.chars().count() > MAX_TOOL_INPUT_CHARS {
        preview.push_str("...");
    }
    preview
}

fn parse_annotation_response(text: &str) -> Result<AnnotationResponse, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Model returned an empty annotation response".to_string());
    }

    if let Ok(payload) = serde_json::from_str::<AnnotationResponse>(trimmed) {
        return validate_annotation_response(payload);
    }

    if let Some(json_body) = extract_json_body(trimmed)
        && let Ok(payload) = serde_json::from_str::<AnnotationResponse>(&json_body)
    {
        return validate_annotation_response(payload);
    }

    Err(format!(
        "Model response was not valid annotation JSON: {}",
        trimmed
    ))
}

fn validate_annotation_response(payload: AnnotationResponse) -> Result<AnnotationResponse, String> {
    if payload.comment.trim().is_empty() {
        return Err("Model returned an empty annotation comment".to_string());
    }
    Ok(payload)
}

fn extract_json_body(text: &str) -> Option<String> {
    let fenced = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .map(str::trim_start)?;
    let unfenced = fenced.strip_suffix("```")?.trim();
    Some(unfenced.to_string())
}

fn absolutize_path(path: &str, cwd: &Path) -> String {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_string_lossy().to_string()
    } else {
        cwd.join(candidate).to_string_lossy().to_string()
    }
}

fn derive_workspace_root(input: &str, source_roots: &[String], fallback: &Path) -> PathBuf {
    let mut candidates = Vec::new();
    candidates.push(path_root_candidate(Path::new(input)));
    for root in source_roots {
        candidates.push(path_root_candidate(Path::new(root)));
    }

    common_ancestor(candidates.into_iter().flatten().collect::<Vec<_>>())
        .unwrap_or_else(|| fallback.to_path_buf())
}

fn path_root_candidate(path: &Path) -> Option<PathBuf> {
    let absolute = fs::canonicalize(path).ok().or_else(|| {
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            None
        }
    })?;

    if absolute.is_dir() {
        Some(absolute)
    } else {
        absolute.parent().map(Path::to_path_buf)
    }
}

fn common_ancestor(paths: Vec<PathBuf>) -> Option<PathBuf> {
    let mut iter = paths.into_iter();
    let first = iter.next()?;
    let mut current = first;

    for path in iter {
        let mut next = current.clone();
        while !path.starts_with(&next) {
            if !next.pop() {
                return None;
            }
        }
        current = next;
    }

    Some(current)
}

fn display_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .map(|relative| relative.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn lang_matches(left: &str, right: &str) -> bool {
    normalize_lang(left) == normalize_lang(right)
}

fn normalize_lang(lang: &str) -> String {
    lang.trim().replace('_', "-").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mentra::{
        BuiltinProvider, ModelInfo, ProviderDescriptor,
        provider::{
            ContentBlockDelta, ContentBlockStart, Provider, ProviderEvent, ProviderEventStream,
            Request, Response, Role, provider_event_stream_from_response,
        },
        runtime::RunOptions,
    };
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    struct FakeBackend {
        responses: HashMap<String, Option<AnnotationResponse>>,
    }

    #[async_trait]
    impl AnnotationBackend for FakeBackend {
        async fn annotate(
            &self,
            request: AnnotationRequest,
        ) -> Result<Option<AnnotationResponse>, String> {
            Ok(self.responses.get(&request.key).cloned().flatten())
        }
    }

    struct RuntimeHoldingBackend {
        _runtime: Arc<tokio::runtime::Runtime>,
    }

    #[async_trait]
    impl AnnotationBackend for RuntimeHoldingBackend {
        async fn annotate(
            &self,
            _request: AnnotationRequest,
        ) -> Result<Option<AnnotationResponse>, String> {
            Ok(Some(AnnotationResponse {
                comment: "Generated comment".to_string(),
                confidence: "high".to_string(),
            }))
        }
    }

    struct RecordingProvider {
        requests: Arc<Mutex<Vec<Request<'static>>>>,
    }

    struct ScriptedStreamingProvider {
        requests: Arc<Mutex<Vec<Request<'static>>>>,
        scripts: Arc<Mutex<VecDeque<Vec<ProviderEvent>>>>,
    }

    #[async_trait]
    impl Provider for RecordingProvider {
        fn descriptor(&self) -> ProviderDescriptor {
            ProviderDescriptor::new(BuiltinProvider::OpenAI)
        }

        async fn list_models(&self) -> Result<Vec<ModelInfo>, mentra::provider::ProviderError> {
            Ok(vec![ModelInfo::new("test-model", BuiltinProvider::OpenAI)])
        }

        async fn stream(
            &self,
            request: Request<'_>,
        ) -> Result<ProviderEventStream, mentra::provider::ProviderError> {
            self.requests
                .lock()
                .expect("requests lock")
                .push(request.clone().into_owned());
            Ok(provider_event_stream_from_response(Response {
                id: "resp-1".to_string(),
                model: request.model.to_string(),
                role: Role::Assistant,
                content: vec![ContentBlock::text(
                    r#"{"comment":"A button label that starts the game.","confidence":"high"}"#,
                )],
                stop_reason: Some("end_turn".to_string()),
                usage: None,
            }))
        }
    }

    #[async_trait]
    impl Provider for ScriptedStreamingProvider {
        fn descriptor(&self) -> ProviderDescriptor {
            ProviderDescriptor::new(BuiltinProvider::OpenAI)
        }

        async fn list_models(&self) -> Result<Vec<ModelInfo>, mentra::provider::ProviderError> {
            Ok(vec![ModelInfo::new("test-model", BuiltinProvider::OpenAI)])
        }

        async fn stream(
            &self,
            request: Request<'_>,
        ) -> Result<ProviderEventStream, mentra::provider::ProviderError> {
            self.requests
                .lock()
                .expect("requests lock")
                .push(request.clone().into_owned());
            let script = self
                .scripts
                .lock()
                .expect("scripts lock")
                .pop_front()
                .expect("missing scripted response");

            let (tx, rx) = mpsc::unbounded_channel();
            for event in script {
                tx.send(Ok(event)).expect("send provider event");
            }
            Ok(rx)
        }
    }

    #[test]
    fn build_agent_config_limits_tools_to_files() {
        let config = build_agent_config(Path::new("/tmp/project"));
        assert!(config.tool_profile.allows("files"));
        assert!(config.tool_profile.allows("shell"));
        assert!(!config.tool_profile.allows("task"));
    }

    #[test]
    fn parse_annotation_response_accepts_fenced_json() {
        let parsed = parse_annotation_response(
            "```json\n{\"comment\":\"Dialog title for room exit confirmation.\",\"confidence\":\"medium\"}\n```",
        )
        .expect("parse response");
        assert_eq!(
            parsed,
            AnnotationResponse {
                comment: "Dialog title for room exit confirmation.".to_string(),
                confidence: "medium".to_string(),
            }
        );
    }

    #[test]
    fn run_annotate_updates_missing_and_auto_generated_comments_only() {
        let temp_dir = TempDir::new().expect("temp dir");
        let input = temp_dir.path().join("Localizable.xcstrings");
        let source_root = temp_dir.path().join("Sources");
        fs::create_dir_all(&source_root).expect("create root");
        fs::write(
            source_root.join("GameView.swift"),
            r#"Text("Start", bundle: .module)"#,
        )
        .expect("write swift");
        fs::write(
            &input,
            r#"{
  "sourceLanguage": "en",
  "version": "1.0",
  "strings": {
    "start": {
      "localizations": {
        "en": { "stringUnit": { "state": "translated", "value": "Start" } }
      }
    },
    "cancel": {
      "comment": "Written by a human.",
      "localizations": {
        "en": { "stringUnit": { "state": "translated", "value": "Cancel" } }
      }
    },
    "retry": {
      "comment": "Old auto comment",
      "isCommentAutoGenerated": true,
      "localizations": {
        "en": { "stringUnit": { "state": "translated", "value": "Retry" } }
      }
    }
  }
}"#,
        )
        .expect("write xcstrings");

        let mut responses = HashMap::new();
        responses.insert(
            "start".to_string(),
            Some(AnnotationResponse {
                comment: "A button label that starts the game.".to_string(),
                confidence: "high".to_string(),
            }),
        );
        responses.insert(
            "retry".to_string(),
            Some(AnnotationResponse {
                comment: "A button label shown when the user can try the action again.".to_string(),
                confidence: "high".to_string(),
            }),
        );

        let opts = ResolvedAnnotateOptions {
            input: input.to_string_lossy().to_string(),
            output: input.to_string_lossy().to_string(),
            source_roots: vec![source_root.to_string_lossy().to_string()],
            source_lang: Some("en".to_string()),
            provider: ProviderKind::OpenAI,
            model: "test-model".to_string(),
            concurrency: 1,
            dry_run: false,
            check: false,
            workspace_root: temp_dir.path().to_path_buf(),
        };

        run_annotate_with_backend(opts, Arc::new(FakeBackend { responses }))
            .expect("annotate command");

        let payload = serde_json::from_str::<serde_json::Value>(
            &fs::read_to_string(&input).expect("read output"),
        )
        .expect("parse output");

        assert_eq!(
            payload["strings"]["start"]["comment"],
            serde_json::Value::String("A button label that starts the game.".to_string())
        );
        assert_eq!(
            payload["strings"]["start"]["isCommentAutoGenerated"],
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            payload["strings"]["retry"]["comment"],
            serde_json::Value::String(
                "A button label shown when the user can try the action again.".to_string()
            )
        );
        assert_eq!(
            payload["strings"]["cancel"]["comment"],
            serde_json::Value::String("Written by a human.".to_string())
        );
    }

    #[test]
    fn run_annotate_dry_run_does_not_write_changes() {
        let temp_dir = TempDir::new().expect("temp dir");
        let input = temp_dir.path().join("Localizable.xcstrings");
        let source_root = temp_dir.path().join("Sources");
        fs::create_dir_all(&source_root).expect("create root");
        fs::write(
            &input,
            r#"{
  "sourceLanguage": "en",
  "version": "1.0",
  "strings": {
    "start": {
      "localizations": {
        "en": { "stringUnit": { "state": "translated", "value": "Start" } }
      }
    }
  }
}"#,
        )
        .expect("write xcstrings");

        let original = fs::read_to_string(&input).expect("read original");
        let mut responses = HashMap::new();
        responses.insert(
            "start".to_string(),
            Some(AnnotationResponse {
                comment: "A button label that starts the game.".to_string(),
                confidence: "high".to_string(),
            }),
        );

        let opts = ResolvedAnnotateOptions {
            input: input.to_string_lossy().to_string(),
            output: input.to_string_lossy().to_string(),
            source_roots: vec![source_root.to_string_lossy().to_string()],
            source_lang: Some("en".to_string()),
            provider: ProviderKind::OpenAI,
            model: "test-model".to_string(),
            concurrency: 1,
            dry_run: true,
            check: false,
            workspace_root: temp_dir.path().to_path_buf(),
        };

        run_annotate_with_backend(opts, Arc::new(FakeBackend { responses }))
            .expect("annotate command");

        assert_eq!(fs::read_to_string(&input).expect("read output"), original);
    }

    #[test]
    fn run_annotate_check_fails_when_changes_would_be_written() {
        let temp_dir = TempDir::new().expect("temp dir");
        let input = temp_dir.path().join("Localizable.xcstrings");
        let source_root = temp_dir.path().join("Sources");
        fs::create_dir_all(&source_root).expect("create root");
        fs::write(
            &input,
            r#"{
  "sourceLanguage": "en",
  "version": "1.0",
  "strings": {
    "start": {
      "localizations": {
        "en": { "stringUnit": { "state": "translated", "value": "Start" } }
      }
    }
  }
}"#,
        )
        .expect("write xcstrings");

        let mut responses = HashMap::new();
        responses.insert(
            "start".to_string(),
            Some(AnnotationResponse {
                comment: "A button label that starts the game.".to_string(),
                confidence: "high".to_string(),
            }),
        );

        let opts = ResolvedAnnotateOptions {
            input: input.to_string_lossy().to_string(),
            output: input.to_string_lossy().to_string(),
            source_roots: vec![source_root.to_string_lossy().to_string()],
            source_lang: Some("en".to_string()),
            provider: ProviderKind::OpenAI,
            model: "test-model".to_string(),
            concurrency: 1,
            dry_run: false,
            check: true,
            workspace_root: temp_dir.path().to_path_buf(),
        };

        let error = run_annotate_with_backend(opts, Arc::new(FakeBackend { responses }))
            .expect_err("check mode should fail");
        assert!(error.contains("would change"));
    }

    #[test]
    fn annotate_requests_does_not_drop_backend_runtime_inside_async_context() {
        let requests = vec![AnnotationRequest {
            key: "start".to_string(),
            source_lang: "en".to_string(),
            source_value: "Start".to_string(),
            existing_comment: None,
            source_roots: vec!["Sources".to_string()],
        }];
        let backend: Arc<dyn AnnotationBackend> = Arc::new(RuntimeHoldingBackend {
            _runtime: Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("build nested runtime"),
            ),
        });
        let mut progress = AnnotateProgressRenderer::new();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            annotate_requests(requests, Arc::clone(&backend), 1, &mut progress)
        }));

        assert!(result.is_ok(), "annotate_requests should not panic");
        let annotations = result.expect("no panic").expect("annotation results");
        assert_eq!(annotations.len(), 1);
        assert!(annotations["start"].is_some());
    }

    #[test]
    fn resolve_annotate_options_uses_config_relative_paths_and_shared_ai_defaults() {
        let temp_dir = TempDir::new().expect("temp dir");
        let project_dir = temp_dir.path().join("project");
        let sources_dir = project_dir.join("Sources");
        let modules_dir = project_dir.join("Modules");
        fs::create_dir_all(&sources_dir).expect("create Sources");
        fs::create_dir_all(&modules_dir).expect("create Modules");
        let input = project_dir.join("Localizable.xcstrings");
        fs::write(
            &input,
            r#"{
  "sourceLanguage": "en",
  "version": "1.0",
  "strings": {}
}"#,
        )
        .expect("write xcstrings");

        let config_path = project_dir.join("langcodec.toml");
        fs::write(
            &config_path,
            r#"[ai]
provider = "openai"
model = "gpt-4.1-mini"

[annotate]
input = "Localizable.xcstrings"
source_roots = ["Sources", "Modules"]
output = "Annotated.xcstrings"
source_lang = "en"
concurrency = 2
"#,
        )
        .expect("write config");

        let loaded = load_config(Some(config_path.to_str().expect("config path")))
            .expect("load config")
            .expect("config present");

        let resolved = resolve_annotate_options(
            &AnnotateOptions {
                input: None,
                source_roots: Vec::new(),
                output: None,
                source_lang: None,
                provider: None,
                model: None,
                concurrency: None,
                config: Some(config_path.to_string_lossy().to_string()),
                dry_run: false,
                check: false,
            },
            Some(&loaded),
        )
        .expect("resolve annotate options");

        assert_eq!(resolved.input, input.to_string_lossy().to_string());
        assert_eq!(
            resolved.output,
            project_dir
                .join("Annotated.xcstrings")
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(
            resolved.source_roots,
            vec![
                sources_dir.to_string_lossy().to_string(),
                modules_dir.to_string_lossy().to_string()
            ]
        );
        assert_eq!(resolved.source_lang.as_deref(), Some("en"));
        assert_eq!(resolved.provider, ProviderKind::OpenAI);
        assert_eq!(resolved.model, "gpt-4.1-mini");
        assert_eq!(resolved.concurrency, 2);
    }

    #[test]
    fn resolve_annotate_options_prefers_cli_over_config() {
        let temp_dir = TempDir::new().expect("temp dir");
        let project_dir = temp_dir.path().join("project");
        let config_sources_dir = project_dir.join("Sources");
        let cli_sources_dir = project_dir.join("AppSources");
        fs::create_dir_all(&config_sources_dir).expect("create config Sources");
        fs::create_dir_all(&cli_sources_dir).expect("create cli Sources");
        let config_input = project_dir.join("Localizable.xcstrings");
        let cli_input = project_dir.join("Runtime.xcstrings");
        fs::write(
            &config_input,
            r#"{
  "sourceLanguage": "en",
  "version": "1.0",
  "strings": {}
}"#,
        )
        .expect("write config xcstrings");
        fs::write(
            &cli_input,
            r#"{
  "sourceLanguage": "en",
  "version": "1.0",
  "strings": {}
}"#,
        )
        .expect("write cli xcstrings");

        let config_path = project_dir.join("langcodec.toml");
        fs::write(
            &config_path,
            r#"[ai]
provider = "openai"
model = "gpt-4.1-mini"

[annotate]
input = "Localizable.xcstrings"
source_roots = ["Sources"]
source_lang = "en"
concurrency = 2
"#,
        )
        .expect("write config");

        let loaded = load_config(Some(config_path.to_str().expect("config path")))
            .expect("load config")
            .expect("config present");

        let resolved = resolve_annotate_options(
            &AnnotateOptions {
                input: Some(cli_input.to_string_lossy().to_string()),
                source_roots: vec![cli_sources_dir.to_string_lossy().to_string()],
                output: Some(
                    project_dir
                        .join("Output.xcstrings")
                        .to_string_lossy()
                        .to_string(),
                ),
                source_lang: Some("fr".to_string()),
                provider: Some("anthropic".to_string()),
                model: Some("claude-sonnet".to_string()),
                concurrency: Some(6),
                config: Some(config_path.to_string_lossy().to_string()),
                dry_run: true,
                check: true,
            },
            Some(&loaded),
        )
        .expect("resolve annotate options");

        assert_eq!(resolved.input, cli_input.to_string_lossy().to_string());
        assert_eq!(
            resolved.source_roots,
            vec![cli_sources_dir.to_string_lossy().to_string()]
        );
        assert_eq!(resolved.source_lang.as_deref(), Some("fr"));
        assert_eq!(resolved.provider, ProviderKind::Anthropic);
        assert_eq!(resolved.model, "claude-sonnet");
        assert_eq!(resolved.concurrency, 6);
        assert!(resolved.dry_run);
        assert!(resolved.check);
    }

    #[test]
    fn expand_annotate_invocations_supports_multiple_config_inputs() {
        let temp_dir = TempDir::new().expect("temp dir");
        let project_dir = temp_dir.path().join("project");
        let sources_dir = project_dir.join("Sources");
        fs::create_dir_all(&sources_dir).expect("create Sources");
        let first = project_dir.join("First.xcstrings");
        let second = project_dir.join("Second.xcstrings");
        fs::write(
            &first,
            r#"{"sourceLanguage":"en","version":"1.0","strings":{}}"#,
        )
        .expect("write first");
        fs::write(
            &second,
            r#"{"sourceLanguage":"en","version":"1.0","strings":{}}"#,
        )
        .expect("write second");

        let config_path = project_dir.join("langcodec.toml");
        fs::write(
            &config_path,
            r#"[ai]
provider = "openai"
model = "gpt-4.1-mini"

[annotate]
inputs = ["First.xcstrings", "Second.xcstrings"]
source_roots = ["Sources"]
source_lang = "en"
concurrency = 2
"#,
        )
        .expect("write config");

        let loaded = load_config(Some(config_path.to_str().expect("config path")))
            .expect("load config")
            .expect("config present");

        let runs = expand_annotate_invocations(
            &AnnotateOptions {
                input: None,
                source_roots: Vec::new(),
                output: None,
                source_lang: None,
                provider: None,
                model: None,
                concurrency: None,
                config: Some(config_path.to_string_lossy().to_string()),
                dry_run: false,
                check: false,
            },
            Some(&loaded),
        )
        .expect("expand annotate invocations");

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].input, first.to_string_lossy().to_string());
        assert_eq!(runs[1].input, second.to_string_lossy().to_string());
        assert_eq!(
            runs[0].source_roots,
            vec![sources_dir.to_string_lossy().to_string()]
        );
        assert_eq!(
            runs[1].source_roots,
            vec![sources_dir.to_string_lossy().to_string()]
        );
    }

    #[test]
    fn expand_annotate_invocations_rejects_input_and_inputs_together() {
        let temp_dir = TempDir::new().expect("temp dir");
        let config_path = temp_dir.path().join("langcodec.toml");
        fs::write(
            &config_path,
            r#"[annotate]
input = "Localizable.xcstrings"
inputs = ["One.xcstrings", "Two.xcstrings"]
source_roots = ["Sources"]
"#,
        )
        .expect("write config");

        let loaded = load_config(Some(config_path.to_str().expect("config path")))
            .expect("load config")
            .expect("config present");

        let err = expand_annotate_invocations(
            &AnnotateOptions {
                input: None,
                source_roots: Vec::new(),
                output: None,
                source_lang: None,
                provider: None,
                model: None,
                concurrency: None,
                config: Some(config_path.to_string_lossy().to_string()),
                dry_run: false,
                check: false,
            },
            Some(&loaded),
        )
        .expect_err("expected conflicting config to fail");

        assert!(err.contains("annotate.input and annotate.inputs"));
    }

    #[test]
    fn expand_annotate_invocations_rejects_shared_output_for_multiple_inputs() {
        let temp_dir = TempDir::new().expect("temp dir");
        let project_dir = temp_dir.path().join("project");
        let sources_dir = project_dir.join("Sources");
        fs::create_dir_all(&sources_dir).expect("create Sources");
        fs::write(
            project_dir.join("One.xcstrings"),
            r#"{"sourceLanguage":"en","version":"1.0","strings":{}}"#,
        )
        .expect("write One");
        fs::write(
            project_dir.join("Two.xcstrings"),
            r#"{"sourceLanguage":"en","version":"1.0","strings":{}}"#,
        )
        .expect("write Two");

        let config_path = project_dir.join("langcodec.toml");
        fs::write(
            &config_path,
            r#"[ai]
provider = "openai"
model = "gpt-4.1-mini"

[annotate]
inputs = ["One.xcstrings", "Two.xcstrings"]
source_roots = ["Sources"]
output = "Annotated.xcstrings"
"#,
        )
        .expect("write config");

        let loaded = load_config(Some(config_path.to_str().expect("config path")))
            .expect("load config")
            .expect("config present");

        let err = expand_annotate_invocations(
            &AnnotateOptions {
                input: None,
                source_roots: Vec::new(),
                output: None,
                source_lang: None,
                provider: None,
                model: None,
                concurrency: None,
                config: Some(config_path.to_string_lossy().to_string()),
                dry_run: false,
                check: false,
            },
            Some(&loaded),
        )
        .expect_err("expected multiple input/output conflict");

        assert!(err.contains("annotate.inputs cannot be combined"));
    }

    #[tokio::test]
    async fn mentra_backend_requests_files_tool() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let provider = RecordingProvider {
            requests: Arc::clone(&requests),
        };
        let runtime = Runtime::builder()
            .with_provider_instance(provider)
            .build()
            .expect("build runtime");
        let backend = MentraAnnotatorBackend::from_runtime(
            runtime,
            ModelInfo::new("test-model", BuiltinProvider::OpenAI),
            PathBuf::from("/tmp/project"),
        );

        let response = backend
            .annotate(AnnotationRequest {
                key: "start".to_string(),
                source_lang: "en".to_string(),
                source_value: "Start".to_string(),
                existing_comment: None,
                source_roots: vec!["Sources".to_string()],
            })
            .await
            .expect("annotate")
            .expect("response");

        assert_eq!(response.comment, "A button label that starts the game.");
        let recorded = requests.lock().expect("requests lock");
        assert_eq!(recorded.len(), 1);
        let tool_names = recorded[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>();
        assert!(tool_names.contains(&"files"));
        assert!(tool_names.contains(&"shell"));
    }

    #[tokio::test]
    async fn old_tool_enabled_annotate_flow_recovers_from_malformed_tool_json_on_mentra_030() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let scripts = VecDeque::from([
            vec![
                ProviderEvent::MessageStarted {
                    id: "msg-1".to_string(),
                    model: "test-model".to_string(),
                    role: Role::Assistant,
                },
                ProviderEvent::ContentBlockStarted {
                    index: 0,
                    kind: ContentBlockStart::ToolUse {
                        id: "tool-1".to_string(),
                        name: "files".to_string(),
                    },
                },
                ProviderEvent::ContentBlockDelta {
                    index: 0,
                    delta: ContentBlockDelta::ToolUseInputJson(
                        r#"{"path":"Sources/GameView.swift"#.to_string(),
                    ),
                },
                ProviderEvent::ContentBlockStopped { index: 0 },
                ProviderEvent::MessageStopped,
            ],
            Response {
                id: "resp-2".to_string(),
                model: "test-model".to_string(),
                role: Role::Assistant,
                content: vec![ContentBlock::text(
                    r#"{"comment":"A button label that starts the game.","confidence":"high"}"#,
                )],
                stop_reason: Some("end_turn".to_string()),
                usage: None,
            }
            .into_provider_events(),
        ]);
        let provider = ScriptedStreamingProvider {
            requests: Arc::clone(&requests),
            scripts: Arc::new(Mutex::new(scripts)),
        };
        let runtime = Runtime::builder()
            .with_provider_instance(provider)
            .build()
            .expect("build runtime");
        let mut agent = runtime
            .spawn_with_config(
                "annotate",
                ModelInfo::new("test-model", BuiltinProvider::OpenAI),
                build_agent_config(Path::new("/tmp/project")),
            )
            .expect("spawn agent");
        let request = AnnotationRequest {
            key: "start".to_string(),
            source_lang: "en".to_string(),
            source_value: "Start".to_string(),
            existing_comment: None,
            source_roots: vec!["Sources".to_string()],
        };

        let response = agent
            .run(
                vec![ContentBlock::text(build_annotation_prompt(&request))],
                RunOptions {
                    tool_budget: Some(DEFAULT_TOOL_BUDGET),
                    ..RunOptions::default()
                },
            )
            .await
            .expect("run annotate");
        let parsed = parse_annotation_response(&response.text()).expect("parse annotation");

        assert_eq!(parsed.comment, "A button label that starts the game.");
        let recorded = requests.lock().expect("requests lock");
        assert_eq!(recorded.len(), 2);
        assert!(
            recorded[1]
                .messages
                .iter()
                .flat_map(|message| message.content.iter())
                .any(|block| matches!(block, ContentBlock::Text { text } if text.contains("One or more tool calls could not be executed because their JSON arguments were invalid.")))
        );
    }
}
