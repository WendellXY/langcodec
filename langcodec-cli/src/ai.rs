use std::sync::Arc;

use mentra::{BuiltinProvider, provider::{self, Provider}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProviderKind {
    OpenAI,
    Anthropic,
    Gemini,
}

impl ProviderKind {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
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

    pub(crate) fn display_name(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
        }
    }

    pub(crate) fn api_key_env(&self) -> &'static str {
        match self {
            Self::OpenAI => "OPENAI_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::Gemini => "GEMINI_API_KEY",
        }
    }

    pub(crate) fn builtin_provider(&self) -> BuiltinProvider {
        match self {
            Self::OpenAI => BuiltinProvider::OpenAI,
            Self::Anthropic => BuiltinProvider::Anthropic,
            Self::Gemini => BuiltinProvider::Gemini,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ProviderSetup {
    pub(crate) provider_kind: ProviderKind,
    pub(crate) provider: Arc<dyn Provider>,
}

pub(crate) fn resolve_provider(
    cli: Option<&str>,
    shared_cfg: Option<&str>,
    legacy_cfg: Option<&str>,
) -> Result<ProviderKind, String> {
    if let Some(value) = cli {
        return ProviderKind::parse(value);
    }
    if let Some(value) = shared_cfg {
        return ProviderKind::parse(value);
    }
    if let Some(value) = legacy_cfg {
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
            "--provider is required (or set ai.provider in langcodec.toml, or use legacy translate.provider, or configure exactly one provider API key)"
                .to_string(),
        ),
        _ => Err(
            "Multiple provider API keys are configured; specify --provider or set ai.provider in langcodec.toml"
                .to_string(),
        ),
    }
}

pub(crate) fn resolve_model(
    cli: Option<&str>,
    shared_cfg: Option<&str>,
    legacy_cfg: Option<&str>,
) -> Result<String, String> {
    cli.map(ToOwned::to_owned)
        .or_else(|| shared_cfg.map(ToOwned::to_owned))
        .or_else(|| legacy_cfg.map(ToOwned::to_owned))
        .or_else(|| std::env::var("MENTRA_MODEL").ok())
        .ok_or_else(|| {
            "--model is required (or set ai.model in langcodec.toml, or use legacy translate.model, or set MENTRA_MODEL)"
                .to_string()
        })
}

pub(crate) fn read_api_key(kind: &ProviderKind) -> Result<String, String> {
    std::env::var(kind.api_key_env()).map_err(|_| {
        format!(
            "Missing {} environment variable for {} provider",
            kind.api_key_env(),
            kind.display_name()
        )
    })
}

pub(crate) fn build_provider(kind: &ProviderKind) -> Result<ProviderSetup, String> {
    let api_key = read_api_key(kind)?;

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
