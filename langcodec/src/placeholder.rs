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
                out.push(PlaceholderToken {
                    index,
                    kind: canonical_kind_char(ch),
                });
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
    // Replace positional iOS object placeholders %<n>$@ -> %<n>$s
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut tmp = String::with_capacity(input.len());
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let mut j = i + 1;
            let start_digits = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > start_digits && j + 1 < bytes.len() && bytes[j] == b'$' && bytes[j + 1] == b'@' {
                // Copy prefix, then normalized token
                tmp.push('%');
                tmp.push_str(&input[start_digits..j]); // digits
                tmp.push('$');
                tmp.push('s');
                i = j + 2;
                continue;
            }
        }
        tmp.push(bytes[i] as char);
        i += 1;
    }

    // Simple iOS object -> string
    let out = tmp.replace("%@", "%s");
    // Long ints to canonical
    let out = out.replace("%ld", "%d");

    out.replace("%lu", "%u")
}

/// Convert canonical/Android-style string placeholders to iOS-style.
/// - %s   -> %@
/// - %1$s -> %1$@
///   Leaves numeric specifiers (e.g., %d, %u, %ld) unchanged.
pub fn to_ios_placeholders(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut i = 0usize;
    let mut out = String::with_capacity(input.len());
    while i < bytes.len() {
        if bytes[i] != b'%' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        // Escaped percent '%%'
        if i + 1 < bytes.len() && bytes[i + 1] == b'%' {
            out.push('%');
            out.push('%');
            i += 2;
            continue;
        }

        // Examine potential placeholder
        let mut j = i + 1;
        // Optional positional index digits+$
        let start_digits = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        let mut had_positional = false;
        if j > start_digits && j < bytes.len() && bytes[j] == b'$' {
            had_positional = true;
            j += 1; // skip '$'
        } else {
            // reset if not positional
            j = i + 1;
        }

        // Optional length modifiers (l/ll). We will drop them when converting %s -> %@.
        let mut k = j;
        while k < bytes.len() && bytes[k] == b'l' {
            k += 1;
        }
        if k >= bytes.len() {
            // not a complete placeholder, copy '%' and advance
            out.push('%');
            i += 1;
            continue;
        }

        let ty = bytes[k] as char;
        if ty == 's' {
            // Emit converted iOS placeholder
            out.push('%');
            if had_positional {
                // copy the digits we saw
                out.push_str(
                    &input[start_digits..(if start_digits < j {
                        j - 1
                    } else {
                        start_digits
                    })],
                );
                out.push('$');
            }
            out.push('@');
            i = k + 1;
            continue;
        }

        // Not a string placeholder, emit one byte and continue (simple path)
        out.push('%');
        i += 1;
    }
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
    fn test_normalize_positional_object() {
        let s = "Hello %1$@";
        let n = normalize_placeholders(s);
        assert!(n.contains("%1$s"));
    }

    #[test]
    fn test_ignore_escaped_percent() {
        let s = "Discount: 50%% and value %d";
        let sig = signature(s);
        assert_eq!(sig, vec!["d"]);
    }
}
