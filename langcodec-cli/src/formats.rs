use std::str::FromStr;

/// Custom format types that are not supported by the lib crate.
/// These are one-way conversions only (to Resource format).
#[derive(Debug, Clone, PartialEq, clap::ValueEnum)]
pub enum CustomFormat {
    /// A JSON file which contains a map of language codes to translations.
    ///
    /// The key is the localization code, and the value is the translation:
    ///
    /// ```json
    /// {
    ///     "key": "hello_world",
    ///     "en": "Hello, World!",
    ///     "fr": "Bonjour, le monde!"
    /// }
    /// ```
    JSONLanguageMap,

    /// A YAML file which contains a map of language codes to translations.
    ///
    /// The key is the localization code, and the value is the translation:
    ///
    /// ```yaml
    /// key: hello_world
    /// en: Hello, World!
    /// fr: Bonjour, le monde!
    /// ```
    YAMLLanguageMap,

    /// A JSON file which contains an array of language map objects.
    ///
    /// Each object contains a key and translations for different languages:
    ///
    /// ```json
    /// [
    ///     {
    ///         "key": "hello_world",
    ///         "en": "Hello, World!",
    ///         "fr": "Bonjour, le monde!"
    ///     },
    ///     {
    ///         "key": "welcome_message",
    ///         "en": "Welcome to our app!",
    ///         "fr": "Bienvenue dans notre application!"
    ///     }
    /// ]
    /// ```
    JSONArrayLanguageMap,
}

impl FromStr for CustomFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_lowercase().replace(['-', '_'], "");
        //: cspell:disable
        match normalized.as_str() {
            "jsonlanguagemap" => Ok(CustomFormat::JSONLanguageMap),
            "jsonarraylanguagemap" => Ok(CustomFormat::JSONArrayLanguageMap),
            "yamllanguagemap" => Ok(CustomFormat::YAMLLanguageMap),
            // "csvlanguages" => Ok(CustomFormat::CSVLanguages),
            _ => Err(format!(
                "Unknown custom format: '{}'. Supported formats: json-language-map, json-array-language-map, yaml-language-map",
                s
            )),
        }
        //: cspell:enable
    }
}

/// Parse a custom format from a string, with helpful error messages.
pub fn parse_custom_format(s: &str) -> Result<CustomFormat, String> {
    CustomFormat::from_str(s)
}

/// Detect if a file is a custom format based on its content and extension.
/// Returns the detected custom format if found, None otherwise.
pub fn detect_custom_format(file_path: &str, file_content: &str) -> Option<CustomFormat> {
    let extension = std::path::Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "json" => {
            // Try to parse as JSON object first (JSONLanguageMap)
            if serde_json::from_str::<serde_json::Value>(file_content).is_ok() {
                // Check if it's an object (not an array)
                if let Ok(obj) = serde_json::from_str::<
                    std::collections::HashMap<String, serde_json::Value>,
                >(file_content)
                {
                    if !obj.is_empty() {
                        return Some(CustomFormat::JSONLanguageMap);
                    }
                }
                // Check if it's an array (JSONArrayLanguageMap)
                if serde_json::from_str::<Vec<serde_json::Value>>(file_content).is_ok() {
                    return Some(CustomFormat::JSONArrayLanguageMap);
                }
            }
        }
        "yaml" | "yml" => {
            // Try to parse as YAML
            if serde_yaml::from_str::<serde_yaml::Value>(file_content).is_ok() {
                return Some(CustomFormat::YAMLLanguageMap);
            }
        }
        _ => {}
    }

    None
}

/// Get a list of all supported custom formats for help messages.
pub fn get_supported_custom_formats() -> &'static str {
    "json-language-map, json-array-language-map, yaml-language-map"
}
