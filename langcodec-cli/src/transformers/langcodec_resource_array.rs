use std::fs;

/// Transform a langcodec::Resource array JSON file to a Vec<Resource>.
/// This format is a direct representation of langcodec::Resource objects in JSON.
pub fn transform(input: String) -> Result<Vec<langcodec::Resource>, String> {
    // Read the file content
    let content =
        fs::read_to_string(&input).map_err(|e| format!("Error reading file {}: {}", input, e))?;

    // Parse as JSON array of Resource objects
    let resources: Vec<langcodec::Resource> = serde_json::from_str(&content)
        .map_err(|e| format!("Error parsing JSON as Resource array: {}", e))?;

    // Validate that we have at least one resource
    if resources.is_empty() {
        return Err("Resource array is empty".to_string());
    }

    Ok(resources)
}
