pub mod android_strings;
pub mod csv;
pub mod strings;
pub mod xcstrings;

// Reexporting the formats for easier access
pub use android_strings::Format as AndroidStringsFormat;
pub use strings::Format as StringsFormat;
pub use xcstrings::Format as XcstringsFormat;

pub enum FormatType {
    AndroidStrings(Option<String>),
    Strings(Option<String>),
    Xcstrings,
}

impl FormatType {
    pub fn extension(&self) -> &'static str {
        match self {
            FormatType::AndroidStrings(_) => "xml",
            FormatType::Strings(_) => "strings",
            FormatType::Xcstrings => "xcstrings",
        }
    }

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
