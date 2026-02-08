//! Options for strict/controlled file reading into `Resource`.

/// Read behavior options for [`crate::Codec`] file-loading APIs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReadOptions {
    /// Optional language hint applied to all loaded resources.
    pub language_hint: Option<String>,
    /// Enables stricter checks (e.g., language is required for single-language formats).
    pub strict: bool,
    /// Whether to record source provenance fields in `metadata.custom`.
    pub attach_provenance: bool,
}

impl ReadOptions {
    /// Creates default read options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a language hint.
    pub fn with_language_hint(mut self, language_hint: Option<String>) -> Self {
        self.language_hint = language_hint;
        self
    }

    /// Enables/disables strict mode.
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Enables/disables provenance capture.
    pub fn with_provenance(mut self, attach_provenance: bool) -> Self {
        self.attach_provenance = attach_provenance;
        self
    }
}
