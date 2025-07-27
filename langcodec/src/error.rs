//! All error types for the langcodec crate.
//!
//! These are returned from all fallible operations (parsing, serialization, conversion, etc.).

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown format `{0}`")]
    UnknownFormat(String),

    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("XML parse error: {0}")]
    XmlParse(#[from] quick_xml::Error),

    #[error("CSV parse error: {0}")]
    CsvParse(#[from] csv::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid data: {0}")]
    DataMismatch(String),

    #[error("invalid resource: {0}")]
    InvalidResource(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("conversion error: {message}")]
    Conversion {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("validation error: {0}")]
    Validation(String),
}

impl Error {
    /// Creates a new conversion error with optional source error
    pub fn conversion_error(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Error::Conversion {
            message: message.into(),
            source,
        }
    }

    /// Creates a new validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Error::Validation(message.into())
    }
}
