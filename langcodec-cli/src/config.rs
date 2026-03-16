use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CliConfig {
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub translate: TranslateConfig,
    #[serde(default)]
    pub annotate: AnnotateConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AiConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TranslateConfig {
    pub source: Option<String>,
    pub sources: Option<Vec<String>>,
    pub target: Option<String>,
    pub output: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub concurrency: Option<usize>,
    pub status: Option<Vec<String>>,
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
    pub fn shared_provider(&self) -> Option<&str> {
        self.ai
            .provider
            .as_deref()
            .or(self.translate.provider.as_deref())
    }

    pub fn shared_model(&self) -> Option<&str> {
        self.ai.model.as_deref().or(self.translate.model.as_deref())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn cli_config_shared_ai_falls_back_to_translate_values() {
        let config: CliConfig = toml::from_str(
            r#"
[translate]
provider = "openai"
model = "gpt-4.1-mini"
"#,
        )
        .expect("parse config");

        assert_eq!(config.shared_provider(), Some("openai"));
        assert_eq!(config.shared_model(), Some("gpt-4.1-mini"));
    }

    #[test]
    fn cli_config_shared_ai_prefers_ai_section() {
        let config: CliConfig = toml::from_str(
            r#"
[ai]
provider = "anthropic"
model = "claude-sonnet"

[translate]
provider = "openai"
model = "gpt-4.1-mini"
"#,
        )
        .expect("parse config");

        assert_eq!(config.shared_provider(), Some("anthropic"));
        assert_eq!(config.shared_model(), Some("claude-sonnet"));
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
[ai]
provider = "openai"
model = "gpt-4.1-mini"

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
[ai]
provider = "openai"
model = "gpt-4.1-mini"

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
}
