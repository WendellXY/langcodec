//! CLI library for testing purposes

pub mod ai;
pub mod annotate;
pub mod config;
pub mod formats;
pub mod merge;
pub mod tolgee;
pub mod transformers;
pub mod translate;
pub mod tui;
pub mod ui;
pub mod validation;

pub use annotate::{AnnotateOptions, run_annotate_command};
pub use formats::{CustomFormat, parse_custom_format};
pub use langcodec::Codec;
pub use tolgee::{
    TolgeePullOptions, TolgeePushOptions, run_tolgee_pull_command, run_tolgee_push_command,
};
pub use transformers::custom_format_to_resource;
pub use translate::{TranslateOptions, run_translate_command};
pub use tui::UiMode;
