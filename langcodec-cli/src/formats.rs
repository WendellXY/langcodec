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
