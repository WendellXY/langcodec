//! All supported localization file formats for langcodec.
//!
//! This module re-exports the main types for each format and provides
//! the [`FormatType`] enum for generic format handling across the crate.

pub mod android_strings;
pub mod csv;
pub mod strings;
pub mod xcstrings;

use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

// Reexporting the formats for easier access
pub use android_strings::Format as AndroidStringsFormat;
pub use strings::Format as StringsFormat;
pub use xcstrings::Format as XcstringsFormat;

use crate::Error;

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

/// Implements [`std::fmt::Display`] for [`FormatType`].
///
/// This provides a human-friendly string for each format type:
/// - `AndroidStrings(_)` → `"android"`
/// - `Strings(_)` → `"strings"`
/// - `Xcstrings` → `"xcstrings"`
///
/// # Example
/// ```rust
/// use langcodec::formats::FormatType;
/// use std::fmt::Display;
/// assert_eq!(FormatType::AndroidStrings(None).to_string(), "android");
/// assert_eq!(FormatType::Strings(None).to_string(), "strings");
/// assert_eq!(FormatType::Xcstrings.to_string(), "xcstrings");
/// ```
impl Display for FormatType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatType::AndroidStrings(_) => write!(f, "android"),
            FormatType::Strings(_) => write!(f, "strings"),
            FormatType::Xcstrings => write!(f, "xcstrings"),
        }
    }
}

/// Implements [`std::str::FromStr`] for [`FormatType`].
///
/// Accepts the following case-insensitive strings:
/// - `"android"`, `"androidstrings"`, `"xml"` → `FormatType::AndroidStrings(None)`
/// - `"strings"` → `FormatType::Strings(None)`
/// - `"xcstrings"` → `FormatType::Xcstrings`
///
/// Returns [`crate::error::Error::UnknownFormat`] for unknown strings.
///
/// # Example
/// ```rust
/// use langcodec::formats::FormatType;
/// use std::str::FromStr;
/// assert_eq!(FormatType::from_str("android").unwrap(), FormatType::AndroidStrings(None));
/// assert_eq!(FormatType::from_str("strings").unwrap(), FormatType::Strings(None));
/// assert_eq!(FormatType::from_str("xcstrings").unwrap(), FormatType::Xcstrings);
/// assert!(FormatType::from_str("foobar").is_err());
/// ```
impl FromStr for FormatType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_ascii_lowercase();
        match s.as_str() {
            "android" | "androidstrings" | "xml" => Ok(FormatType::AndroidStrings(None)),
            "strings" => Ok(FormatType::Strings(None)),
            "xcstrings" => Ok(FormatType::Xcstrings),
            other => Err(Error::UnknownFormat(other.to_string())),
        }
    }
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
