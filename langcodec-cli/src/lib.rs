//! CLI library for testing purposes

pub mod config;
pub mod formats;
pub mod merge;
pub mod translate;
pub mod transformers;
pub mod validation;

pub use formats::{CustomFormat, parse_custom_format};
pub use langcodec::Codec;
pub use translate::{TranslateOptions, run_translate_command};
pub use transformers::custom_format_to_resource;
