//! CLI library for testing purposes

pub mod ai;
pub mod annotate;
pub mod config;
pub mod formats;
pub mod merge;
pub mod transformers;
pub mod translate;
pub mod validation;

pub use formats::{CustomFormat, parse_custom_format};
pub use langcodec::Codec;
pub use transformers::custom_format_to_resource;
pub use annotate::{AnnotateOptions, run_annotate_command};
pub use translate::{TranslateOptions, run_translate_command};
