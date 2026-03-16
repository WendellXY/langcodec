use std::sync::Arc;

use crate::config::CliConfig;
use mentra::{
    BuiltinProvider,
    provider::{self, Provider},
};

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
    config: Option<&CliConfig>,
    translate_cfg: Option<&str>,
) -> Result<ProviderKind, String> {
    if let Some(value) = cli {
        return ProviderKind::parse(value);
    }
    if let Some(value) = translate_cfg {
        return ProviderKind::parse(value);
    }
    if let Some(config) = config {
        let configured = config.configured_provider_names();
        match configured.len() {
            1 => return ProviderKind::parse(configured[0]),
            0 => {}
            _ => {
                return Err(
                    "Multiple provider sections are configured; specify --provider or set translate.provider in langcodec.toml"
                        .to_string(),
                );
            }
        }
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
            "--provider is required (or configure exactly one provider section like [openai] in langcodec.toml, set translate.provider, or configure exactly one provider API key)"
                .to_string(),
        ),
        _ => Err(
            "Multiple provider API keys are configured; specify --provider or configure a single provider section in langcodec.toml"
                .to_string(),
        ),
    }
}

pub(crate) fn resolve_model(
    cli: Option<&str>,
    config: Option<&CliConfig>,
    provider: &ProviderKind,
    translate_cfg: Option<&str>,
) -> Result<String, String> {
    cli.map(ToOwned::to_owned)
        .or_else(|| {
            config.and_then(|cfg| {
                cfg.provider_model(provider.display_name())
                    .map(ToOwned::to_owned)
            })
        })
        .or_else(|| translate_cfg.map(ToOwned::to_owned))
        .or_else(|| std::env::var("MENTRA_MODEL").ok())
        .ok_or_else(|| {
            format!(
                "--model is required (or set [{}].model in langcodec.toml, set translate.model, or set MENTRA_MODEL)",
                provider.display_name()
            )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_provider_uses_single_configured_provider_section() {
        let config: CliConfig = toml::from_str(
            r#"
[openai]
model = "gpt-5.4"
"#,
        )
        .expect("parse config");

        let provider = resolve_provider(None, Some(&config), None).expect("resolve provider");
        assert_eq!(provider, ProviderKind::OpenAI);
    }

    #[test]
    fn resolve_provider_rejects_multiple_configured_provider_sections() {
        let config: CliConfig = toml::from_str(
            r#"
[openai]
model = "gpt-5.4"

[anthropic]
model = "claude-sonnet"
"#,
        )
        .expect("parse config");

        let err = resolve_provider(None, Some(&config), None).unwrap_err();
        assert!(err.contains("Multiple provider sections are configured"));
    }

    #[test]
    fn resolve_model_prefers_selected_provider_section() {
        let config: CliConfig = toml::from_str(
            r#"
[openai]
model = "gpt-5.4"

[anthropic]
model = "claude-sonnet"
"#,
        )
        .expect("parse config");

        let model = resolve_model(None, Some(&config), &ProviderKind::Anthropic, None)
            .expect("resolve model");
        assert_eq!(model, "claude-sonnet");
    }
}
