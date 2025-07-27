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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_unknown_format_error() {
        let error = Error::UnknownFormat("invalid_format".to_string());
        assert_eq!(error.to_string(), "unknown format `invalid_format`");
    }

    #[test]
    fn test_parse_error() {
        let json_error = serde_json::from_str::<serde_json::Value>("{ invalid json }").unwrap_err();
        let error = Error::Parse(json_error);
        assert!(error.to_string().contains("parse error"));
    }

    #[test]
    fn test_io_error() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let error = Error::Io(io_error);
        assert!(error.to_string().contains("I/O error"));
    }

    #[test]
    fn test_data_mismatch_error() {
        let error = Error::DataMismatch("Invalid data format".to_string());
        assert_eq!(error.to_string(), "invalid data: Invalid data format");
    }

    #[test]
    fn test_invalid_resource_error() {
        let error = Error::InvalidResource("Missing required field".to_string());
        assert_eq!(
            error.to_string(),
            "invalid resource: Missing required field"
        );
    }

    #[test]
    fn test_unsupported_format_error() {
        let error = Error::UnsupportedFormat("xyz".to_string());
        assert_eq!(error.to_string(), "unsupported format: xyz");
    }

    #[test]
    fn test_conversion_error_with_source() {
        let source_error = Box::new(io::Error::new(io::ErrorKind::NotFound, "Source error"));
        let error = Error::conversion_error("Conversion failed", Some(source_error));
        assert!(
            error
                .to_string()
                .contains("conversion error: Conversion failed")
        );
    }

    #[test]
    fn test_conversion_error_without_source() {
        let error = Error::conversion_error("Conversion failed", None);
        assert!(
            error
                .to_string()
                .contains("conversion error: Conversion failed")
        );
    }

    #[test]
    fn test_validation_error() {
        let error = Error::validation_error("Validation failed");
        assert_eq!(error.to_string(), "validation error: Validation failed");
    }

    #[test]
    fn test_error_display() {
        let errors = vec![
            Error::UnknownFormat("test".to_string()),
            Error::DataMismatch("test".to_string()),
            Error::InvalidResource("test".to_string()),
            Error::UnsupportedFormat("test".to_string()),
            Error::Validation("test".to_string()),
        ];

        for error in errors {
            let display = format!("{}", error);
            assert!(!display.is_empty());
            assert!(display.contains("test"));
        }
    }

    #[test]
    fn test_error_debug() {
        let error = Error::UnknownFormat("test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("UnknownFormat"));
        assert!(debug.contains("test"));
    }
}
