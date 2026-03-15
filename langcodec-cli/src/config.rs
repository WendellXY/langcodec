use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CliConfig {
    #[serde(default)]
    pub translate: TranslateConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TranslateConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub concurrency: Option<usize>,
    pub status: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub data: CliConfig,
}

pub fn load_config(explicit_path: Option<&str>) -> Result<Option<LoadedConfig>, String> {
    let path = match explicit_path {
        Some(path) => {
            let resolved = PathBuf::from(path);
            if !resolved.exists() {
                return Err(format!("Config file does not exist: {}", resolved.display()));
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
