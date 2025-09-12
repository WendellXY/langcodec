use crate::formats::CustomFormat;

pub mod json_array_language_map;
pub mod json_language_map;
pub mod langcodec_resource_array;
pub mod yaml_language_map;

/// Convert a custom format to a Resource that can then be processed by the lib crate.
pub fn custom_format_to_resource(
    input: String,
    format: CustomFormat,
) -> Result<Vec<langcodec::Resource>, String> {
    match format {
        CustomFormat::JSONLanguageMap => json_language_map::transform(input),
        CustomFormat::JSONArrayLanguageMap => json_array_language_map::transform(input),
        CustomFormat::YAMLLanguageMap => yaml_language_map::transform(input),
        CustomFormat::LangcodecResourceArray => langcodec_resource_array::transform(input),
    }
}
