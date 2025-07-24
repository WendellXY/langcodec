use std::collections::HashMap;

use langcodec::{Entry, EntryStatus, Metadata, Resource, Translation};

/// Transform a JSON language map file into Resources.
///
/// Expected format:
/// ```json
/// {
///     "key": "hello_world",
///     "en": "Hello, World!",
///     "fr": "Bonjour, le monde!"
/// }
/// ```
pub fn transform(input: String) -> Result<Vec<Resource>, String> {
    let file_content = match std::fs::read_to_string(&input) {
        Ok(content) => content,
        Err(e) => return Err(format!("Error reading file {}: {}", input, e)),
    };

    // Try to parse as JSON key-value pairs
    let json_object: HashMap<String, String> = match serde_json::from_str(&file_content) {
        Ok(obj) => obj,
        Err(e) => {
            return Err(format!(
                "Error parsing JSON from {}: {}. Expected format: {{\"en\": \"Hello\", \"fr\": \"Bonjour\"}}",
                input, e
            ));
        }
    };

    if json_object.is_empty() {
        return Err("Error: JSON object is empty".to_string());
    }

    // Find the localization key
    // Priority: "key" field > "en" field > first field value
    let localization_key = json_object.get("key").unwrap_or(
        &json_object
            .get("en")
            .unwrap_or(&json_object.iter().next().unwrap().1),
    );

    let mut resources = Vec::new();

    for (lang_code, value) in json_object.iter() {
        // Skip the "key" field as it's not a language code
        if lang_code == "key" {
            continue;
        }

        let mut metadata_custom: HashMap<String, String> = HashMap::new();
        metadata_custom.insert("source_language".to_string(), "en".to_string());
        metadata_custom.insert("version".to_string(), "1.0".to_string());
        metadata_custom.insert("format".to_string(), "JSONLanguageMap".to_string());

        let metadata = Metadata {
            language: lang_code.clone(),
            domain: "".to_string(),
            custom: metadata_custom,
        };

        let mut entry_custom = HashMap::new();
        entry_custom.insert("extraction_state".to_string(), "manual".to_string());

        let entry = Entry {
            id: localization_key.clone(),
            value: Translation::Singular(value.clone()),
            status: EntryStatus::NeedsReview,
            comment: None,
            custom: entry_custom,
        };

        resources.push(Resource {
            metadata,
            entries: vec![entry],
        });
    }

    Ok(resources)
}
