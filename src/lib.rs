//! Universal localization file toolkit for Rust.
//!
//! Supports parsing, writing, and converting between Apple `.strings`, `.xcstrings`, Android `strings.xml`, and CSV files.  
//! All conversion happens through the unified `Resource` model.

pub mod codec;
pub mod error;
pub mod formats;
pub mod traits;
pub mod types;

// Re-export most used types for easy consumption
pub use crate::{
    codec::{Codec, convert},
    error::Error,
    formats::FormatType,
    types::{Entry, EntryStatus, Metadata, Plural, PluralCategory, Resource, Translation},
};
