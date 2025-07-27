#![forbid(unsafe_code)]
//! Universal localization file toolkit for Rust.
//!
//! Supports parsing, writing, and converting between Apple `.strings`, `.xcstrings`, Android `strings.xml`, and CSV files.  
//! All conversion happens through the unified `Resource` model.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use langcodec::{Codec, convert_auto};
//!
//! // Convert between formats automatically
//! convert_auto("en.lproj/Localizable.strings", "strings.xml")?;
//!
//! // Or work with the unified Resource model
//! let mut codec = Codec::new();
//! codec.read_file_by_extension("en.lproj/Localizable.strings", None)?;
//! codec.write_to_file()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Supported Formats
//!
//! - **Apple `.strings`**: Traditional iOS/macOS localization files
//! - **Apple `.xcstrings`**: Modern Xcode localization format with plural support
//! - **Android `strings.xml`**: Android resource files
//! - **CSV**: Comma-separated values for simple key-value pairs
//!
//! # Features
//!
//! - ✨ Parse, write, convert, and merge multiple localization file formats
//! - 🦀 Idiomatic, modular, and ergonomic Rust API
//! - 📦 Designed for CLI tools, CI/CD pipelines, and library integration
//! - 🔄 Unified internal model (`Resource`) for lossless format-agnostic processing
//! - 📖 Well-documented, robust error handling and extensible codebase

pub mod codec;
pub mod error;
pub mod formats;
pub mod traits;
pub mod types;

// Re-export most used types for easy consumption
pub use crate::{
    codec::{Codec, convert, convert_auto, infer_format_from_extension},
    error::Error,
    formats::FormatType,
    types::{Entry, EntryStatus, Metadata, Plural, PluralCategory, Resource, Translation},
};
