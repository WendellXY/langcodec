use std::collections::HashMap;

use langcodec::{Entry, EntryStatus, Metadata, Resource, Translation};

/// Transform a JSON array language map file into Resources.
///
/// Expected format:
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
pub fn transform(input: String) -> Result<Vec<Resource>, String> {
    let file_content = match std::fs::read_to_string(&input) {
        Ok(content) => content,
        Err(e) => return Err(format!("Error reading file {}: {}", input, e)),
    };

    // Try to parse as JSON array
    let json_array: Vec<HashMap<String, String>> = match serde_json::from_str(&file_content) {
        Ok(arr) => arr,
        Err(e) => {
            return Err(format!(
                "Error parsing JSON array from {}: {}. Expected format: [{{\"key\": \"hello\", \"en\": \"Hello\", \"fr\": \"Bonjour\"}}]",
                input, e
            ));
        }
    };

    if json_array.is_empty() {
        return Err("Error: JSON array is empty".to_string());
    }

    let mut resources = Vec::new();
    let mut language_resources: HashMap<String, Vec<Entry>> = HashMap::new();

    for (index, entry) in json_array.iter().enumerate() {
        if entry.is_empty() {
            continue;
        }

        // Find the localization key
        // Priority: "key" field > "en" field > first field value
        let localization_key = entry
            .get("key")
            .unwrap_or(entry.get("en").unwrap_or(entry.iter().next().unwrap().1));

        for (lang_code, value) in entry.iter() {
            // Skip the "key" field as it's not a language code
            if lang_code == "key" {
                continue;
            }

            let mut entry_custom = HashMap::new();
            entry_custom.insert("extraction_state".to_string(), "manual".to_string());
            entry_custom.insert("array_index".to_string(), index.to_string());

            let resource_entry = Entry {
                id: localization_key.clone(),
                value: Translation::Singular(value.clone()),
                status: EntryStatus::NeedsReview,
                comment: None,
                custom: entry_custom,
            };

            language_resources
                .entry(lang_code.clone())
                .or_default()
                .push(resource_entry);
        }
    }

    // Convert the grouped entries into Resources
    for (lang_code, entries) in language_resources {
        let mut metadata_custom: HashMap<String, String> = HashMap::new();
        metadata_custom.insert("source_language".to_string(), "en".to_string());
        metadata_custom.insert("version".to_string(), "1.0".to_string());
        metadata_custom.insert("format".to_string(), "JSONArrayLanguageMap".to_string());

        let metadata = Metadata {
            language: lang_code.clone(),
            domain: "".to_string(),
            custom: metadata_custom,
        };

        resources.push(Resource { metadata, entries });
    }

    Ok(resources)
}
