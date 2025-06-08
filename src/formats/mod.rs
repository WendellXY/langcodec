//! All supported localization file formats for langcodec.
//!
//! This module re-exports the main types for each format and provides
//! the [`FormatType`] enum for generic format handling across the crate.

pub mod android_strings;
pub mod csv;
pub mod strings;
pub mod xcstrings;

// Reexporting the formats for easier access
pub use android_strings::Format as AndroidStringsFormat;
pub use strings::Format as StringsFormat;
pub use xcstrings::Format as XcstringsFormat;

/// Represents all supported localization file formats for generic handling.
///
/// This enum allows you to work with any supported file format in a type-safe way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatType {
    /// Android `strings.xml` format, with optional language code.
    AndroidStrings(Option<String>),
    /// Apple `.strings` format, with optional language code.
    Strings(Option<String>),
    /// Apple `.xcstrings` format (no language code).
    Xcstrings,
}

impl FormatType {
    /// Returns the typical file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            FormatType::AndroidStrings(_) => "xml",
            FormatType::Strings(_) => "strings",
            FormatType::Xcstrings => "xcstrings",
        }
    }

    /// Returns the language code for this format, if available.
    pub fn language(&self) -> Option<&String> {
        match self {
            FormatType::AndroidStrings(lang) => lang.as_ref(),
            FormatType::Strings(lang) => lang.as_ref(),
            FormatType::Xcstrings => None,
        }
    }
}

impl ToString for FormatType {
    fn to_string(&self) -> String {
        match self {
            FormatType::AndroidStrings(_) => "AndroidStrings".to_string(),
            FormatType::Strings(_) => "Strings".to_string(),
            FormatType::Xcstrings => "Xcstrings".to_string(),
        }
    }
}
