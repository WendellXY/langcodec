//! CLI library for testing purposes

pub mod formats;
pub mod transformers;

pub use formats::{CustomFormat, parse_custom_format};
pub use transformers::custom_format_to_resource;
