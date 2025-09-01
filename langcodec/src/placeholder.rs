//! Placeholder parsing, normalization and validation utilities.
//!
//! Goals:
//! - Normalize common iOS vs Android placeholder variants to a canonical form.
//! - Extract a placeholder "signature" for comparison across languages.
//! - Validate placeholder consistency per entry (across all languages and plural forms).


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceholderToken {
    pub index: Option<usize>,
    pub kind: char, // canonical kind: s, d, f, etc.
}

impl PlaceholderToken {
    pub fn to_signature(&self) -> String {
        match self.index {
            Some(i) => format!("{}${}", i, self.kind),
            None => format!("{}", self.kind),
        }
    }
}

/// Extracts placeholder tokens from a string and returns them in occurrence order.
/// Handles iOS and Android variants and ignores escaped percent `%%`.
pub fn extract_placeholders(input: &str) -> Vec<PlaceholderToken> {
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();

    while i < bytes.len() {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        // Handle escaped percent
        if i + 1 < bytes.len() && bytes[i + 1] == b'%' {
            i += 2;
            continue;
        }

        let mut j = i + 1;

        // Optional positional index: digits followed by '$'
        let mut index: Option<usize> = None;
        let start_digits = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j < bytes.len() && j > start_digits && bytes[j] == b'$' {
            // parse digits
            if let Some(num) = std::str::from_utf8(&bytes[start_digits..j])
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
            {
                index = Some(num);
            }
            j += 1; // skip '$'
        } else {
            // reset j if not actually positional
            j = i + 1;
        }

        // Optional length modifiers (l/ll)
        if j < bytes.len() && bytes[j] == b'l' {
            j += 1;
            if j < bytes.len() && bytes[j] == b'l' {
                j += 1;
            }
        }

        // Expect a type character
        if j < bytes.len() {
            let ch = bytes[j] as char;
            if ch.is_ascii_alphabetic() || ch == '@' {
                out.push(PlaceholderToken { index, kind: canonical_kind_char(ch) });
                i = j + 1;
                continue;
            }
        }

        // Not a recognized placeholder; skip this '%'
        i += 1;
    }

    out
}

/// Normalize a string by converting iOS-specific tokens to canonical ones.
/// - %@  -> %s
/// - %1$@ -> %1$s
/// - %ld, %lu -> %d / %u
pub fn normalize_placeholders(input: &str) -> String {
    let mut out = input.to_string();
    // Positional iOS object -> Android string
    out = out.replace("%1$@", "%1$s");
    out = out.replace("%2$@", "%2$s");
    out = out.replace("%3$@", "%3$s");
    out = out.replace("%4$@", "%4$s");
    out = out.replace("%5$@", "%5$s");
    // Simple iOS object -> string
    out = out.replace("%@", "%s");
    // Long ints to canonical
    out = out.replace("%ld", "%d");
    out = out.replace("%lu", "%u");
    out
}

/// Build a normalized signature (sequence of tokens) for comparison.
pub fn signature(input: &str) -> Vec<String> {
    extract_placeholders(&normalize_placeholders(input))
        .into_iter()
        .map(|t| t.to_signature())
        .collect()
}

fn canonical_kind_char(ch: char) -> char {
    match ch {
        '@' => 's',
        // Map uppercase to lowercase for type letters where it matters
        c => c.to_ascii_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_android_and_ios() {
        let s = "Hello %1$@, you have %2$d items and %s extra";
        let sig = signature(s);
        assert_eq!(sig, vec!["1$s", "2$d", "s"]);
    }

    #[test]
    fn test_normalize_ios_simple() {
        let s = "Value: %@ and number %ld";
        let n = normalize_placeholders(s);
        assert!(n.contains("%s"));
        assert!(n.contains("%d"));
        assert_eq!(signature(s), vec!["s", "d"]);
    }

    #[test]
    fn test_ignore_escaped_percent() {
        let s = "Discount: 50%% and value %d";
        let sig = signature(s);
        assert_eq!(sig, vec!["d"]);
    }
}
