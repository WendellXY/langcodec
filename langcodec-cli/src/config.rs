use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CliConfig {
    #[serde(default)]
    pub openai: ProviderConfig,
    #[serde(default)]
    pub anthropic: ProviderConfig,
    #[serde(default)]
    pub gemini: ProviderConfig,
    #[serde(default)]
    pub translate: TranslateConfig,
    #[serde(default)]
    pub annotate: AnnotateConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProviderConfig {
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TranslateConfig {
    pub source: Option<String>,
    pub sources: Option<Vec<String>>,
    pub target: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub source_lang: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string_or_vec")]
    pub target_lang: Option<Vec<String>>,
    pub concurrency: Option<usize>,
    pub status: Option<Vec<String>>,
    pub output_status: Option<String>,
    #[serde(default)]
    pub input: TranslateInputConfig,
    pub output: Option<TranslateOutputScope>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TranslateInputConfig {
    pub source: Option<String>,
    pub sources: Option<Vec<String>>,
    pub lang: Option<String>,
    pub status: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TranslateOutputScope {
    Path(String),
    Config(TranslateOutputConfig),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TranslateOutputConfig {
    pub target: Option<String>,
    pub path: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string_or_vec")]
    pub lang: Option<Vec<String>>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AnnotateConfig {
    pub input: Option<String>,
    pub inputs: Option<Vec<String>>,
    pub source_roots: Option<Vec<String>>,
    pub output: Option<String>,
    pub source_lang: Option<String>,
    pub concurrency: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub data: CliConfig,
}

impl LoadedConfig {
    pub fn config_dir(&self) -> Option<&Path> {
        self.path.parent()
    }
}

impl CliConfig {
    pub fn provider_model(&self, provider: &str) -> Option<&str> {
        match provider.trim().to_ascii_lowercase().as_str() {
            "openai" => self.openai.model.as_deref(),
            "anthropic" => self.anthropic.model.as_deref(),
            "gemini" => self.gemini.model.as_deref(),
            _ => None,
        }
    }

    pub fn configured_provider_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.openai.model.is_some() {
            names.push("openai");
        }
        if self.anthropic.model.is_some() {
            names.push("anthropic");
        }
        if self.gemini.model.is_some() {
            names.push("gemini");
        }
        names
    }
}

impl TranslateConfig {
    pub fn resolved_source(&self) -> Option<&str> {
        self.input.source.as_deref().or(self.source.as_deref())
    }

    pub fn resolved_sources(&self) -> Option<&Vec<String>> {
        self.input.sources.as_ref().or(self.sources.as_ref())
    }

    pub fn resolved_source_lang(&self) -> Option<&str> {
        self.input.lang.as_deref().or(self.source_lang.as_deref())
    }

    pub fn resolved_filter_status(&self) -> Option<&Vec<String>> {
        self.input.status.as_ref().or(self.status.as_ref())
    }

    pub fn resolved_target(&self) -> Option<&str> {
        match self.output.as_ref() {
            Some(TranslateOutputScope::Config(config)) => {
                config.target.as_deref().or(self.target.as_deref())
            }
            _ => self.target.as_deref(),
        }
    }

    pub fn resolved_output_path(&self) -> Option<&str> {
        match self.output.as_ref() {
            Some(TranslateOutputScope::Path(path)) => Some(path.as_str()),
            Some(TranslateOutputScope::Config(config)) => config.path.as_deref(),
            None => None,
        }
    }

    pub fn resolved_target_langs(&self) -> Option<&Vec<String>> {
        match self.output.as_ref() {
            Some(TranslateOutputScope::Config(config)) => {
                config.lang.as_ref().or(self.target_lang.as_ref())
            }
            _ => self.target_lang.as_ref(),
        }
    }

    pub fn resolved_output_status(&self) -> Option<&str> {
        match self.output.as_ref() {
            Some(TranslateOutputScope::Config(config)) => {
                config.status.as_deref().or(self.output_status.as_deref())
            }
            _ => self.output_status.as_deref(),
        }
    }
}

pub fn load_config(explicit_path: Option<&str>) -> Result<Option<LoadedConfig>, String> {
    let path = match explicit_path {
        Some(path) => {
            let resolved = PathBuf::from(path);
            if !resolved.exists() {
                return Err(format!(
                    "Config file does not exist: {}",
                    resolved.display()
                ));
            }
            resolved
        }
        None => match discover_config_path()? {
            Some(path) => path,
            None => return Ok(None),
        },
    };

    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config '{}': {}", path.display(), e))?;
    let data: CliConfig = toml::from_str(&text)
        .map_err(|e| format!("Failed to parse config '{}': {}", path.display(), e))?;
    Ok(Some(LoadedConfig { path, data }))
}

fn discover_config_path() -> Result<Option<PathBuf>, String> {
    let mut current = std::env::current_dir()
        .map_err(|e| format!("Failed to determine current directory: {}", e))?;

    loop {
        let candidate = current.join("langcodec.toml");
        if candidate.is_file() {
            return Ok(Some(candidate));
        }

        if !current.pop() {
            return Ok(None);
        }
    }
}

pub fn resolve_config_relative_path(config_dir: Option<&Path>, path: &str) -> String {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return candidate.to_string_lossy().to_string();
    }

    match config_dir {
        Some(dir) => dir.join(candidate).to_string_lossy().to_string(),
        None => candidate.to_string_lossy().to_string(),
    }
}

fn deserialize_optional_string_or_vec<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }

    let value = Option::<StringOrVec>::deserialize(deserializer)?;
    Ok(value.map(|value| match value {
        StringOrVec::String(value) => vec![value],
        StringOrVec::Vec(values) => values,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn cli_config_lists_provider_sections() {
        let config: CliConfig = toml::from_str(
            r#"
[openai]
model = "gpt-5.4"

[anthropic]
model = "claude-sonnet"
"#,
        )
        .expect("parse config");

        assert_eq!(
            config.configured_provider_names(),
            vec!["openai", "anthropic"]
        );
    }

    #[test]
    fn cli_config_reads_provider_specific_models() {
        let config: CliConfig = toml::from_str(
            r#"
[openai]
model = "gpt-5.4"

[anthropic]
model = "claude-sonnet"
"#,
        )
        .expect("parse config");

        assert_eq!(config.provider_model("openai"), Some("gpt-5.4"));
        assert_eq!(config.provider_model("anthropic"), Some("claude-sonnet"));
        assert_eq!(config.provider_model("gemini"), None);
    }

    #[test]
    fn resolve_config_relative_path_uses_config_dir() {
        let resolved = resolve_config_relative_path(
            Some(Path::new("/tmp/project")),
            "locales/Localizable.xcstrings",
        );
        assert_eq!(resolved, "/tmp/project/locales/Localizable.xcstrings");
    }

    #[test]
    fn load_config_parses_annotate_section() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let config_path = temp_dir.path().join("langcodec.toml");
        fs::write(
            &config_path,
            r#"
[openai]
model = "gpt-5.4"

[annotate]
input = "locales/Localizable.xcstrings"
source_roots = ["Sources", "Modules"]
concurrency = 2
"#,
        )
        .expect("write config");

        let loaded = load_config(Some(config_path.to_str().expect("config path")))
            .expect("load config")
            .expect("config present");

        assert_eq!(
            loaded.data.annotate.input.as_deref(),
            Some("locales/Localizable.xcstrings")
        );
        assert_eq!(
            loaded.data.annotate.source_roots,
            Some(vec!["Sources".to_string(), "Modules".to_string()])
        );
        assert_eq!(loaded.data.annotate.concurrency, Some(2));
    }

    #[test]
    fn load_config_parses_annotate_inputs_section() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let config_path = temp_dir.path().join("langcodec.toml");
        fs::write(
            &config_path,
            r#"
[openai]
model = "gpt-5.4"

[annotate]
inputs = ["locales/A.xcstrings", "locales/B.xcstrings"]
source_roots = ["Sources"]
concurrency = 2
"#,
        )
        .expect("write config");

        let loaded = load_config(Some(config_path.to_str().expect("config path")))
            .expect("load config")
            .expect("config present");

        assert_eq!(
            loaded.data.annotate.inputs,
            Some(vec![
                "locales/A.xcstrings".to_string(),
                "locales/B.xcstrings".to_string()
            ])
        );
    }

    #[test]
    fn load_config_parses_translate_target_lang_array() {
        let config: CliConfig = toml::from_str(
            r#"
[translate]
target_lang = ["fr", "de"]
"#,
        )
        .expect("parse config");

        assert_eq!(
            config.translate.target_lang,
            Some(vec!["fr".to_string(), "de".to_string()])
        );
    }

    #[test]
    fn load_config_preserves_legacy_translate_target_lang_string() {
        let config: CliConfig = toml::from_str(
            r#"
[translate]
target_lang = "fr,de"
"#,
        )
        .expect("parse config");

        assert_eq!(
            config.translate.target_lang,
            Some(vec!["fr,de".to_string()])
        );
    }

    #[test]
    fn load_config_parses_nested_translate_input_output_sections() {
        let config: CliConfig = toml::from_str(
            r#"
[translate.input]
source = "locales/Localizable.xcstrings"
lang = "en"
status = ["new", "stale"]

[translate.output]
target = "locales/Translated.xcstrings"
path = "build/Translated.xcstrings"
lang = ["fr", "de"]
status = "translated"
"#,
        )
        .expect("parse config");

        assert_eq!(
            config.translate.resolved_source(),
            Some("locales/Localizable.xcstrings")
        );
        assert_eq!(config.translate.resolved_source_lang(), Some("en"));
        assert_eq!(
            config.translate.resolved_filter_status(),
            Some(&vec!["new".to_string(), "stale".to_string()])
        );
        assert_eq!(
            config.translate.resolved_target(),
            Some("locales/Translated.xcstrings")
        );
        assert_eq!(
            config.translate.resolved_output_path(),
            Some("build/Translated.xcstrings")
        );
        assert_eq!(
            config.translate.resolved_target_langs(),
            Some(&vec!["fr".to_string(), "de".to_string()])
        );
        assert_eq!(
            config.translate.resolved_output_status(),
            Some("translated")
        );
    }
}
