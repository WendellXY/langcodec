use std::collections::HashSet;
use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobSetBuilder};
use ignore::WalkBuilder;
use rayon::prelude::*;

/// Expand possible glob patterns in a list of input strings into concrete file paths.
/// Uses ignore + globset for fast, parallel, .gitignore-aware traversal.
pub fn expand_input_globs(inputs: &Vec<String>) -> Result<Vec<String>, String> {
    fn has_glob_meta(s: &str) -> bool {
        s.bytes().any(|b| matches!(b, b'*' | b'?' | b'[' | b'{'))
    }

    // Extract a static directory prefix before the first glob meta-character
    fn static_prefix_dir(pattern: &str) -> PathBuf {
        let bytes = pattern.as_bytes();
        let mut idx = 0usize;
        while idx < bytes.len() {
            match bytes[idx] {
                b'*' | b'?' | b'[' | b'{' => break,
                _ => idx += 1,
            }
        }
        let prefix = &pattern[..idx];
        let p = Path::new(prefix);
        if p.is_dir() {
            p.to_path_buf()
        } else {
            p.parent()
                .map(|pp| pp.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        }
    }

    // Build one GlobSet for all patterns (literal_separator to avoid '/' matching)
    let mut builder = GlobSetBuilder::new();
    for pat in inputs {
        let glob = GlobBuilder::new(pat)
            .literal_separator(true)
            .build()
            .map_err(|e| format!("Invalid glob pattern '{}': {}", pat, e))?;
        builder.add(glob);
    }
    let set = builder
        .build()
        .map_err(|e| format!("Failed to build glob set: {}", e))?;

    // Collect unique roots to minimize directory walks
    let mut roots: Vec<PathBuf> = Vec::new();
    for pat in inputs {
        let root = if has_glob_meta(pat) {
            static_prefix_dir(pat)
        } else {
            Path::new(pat)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        };
        if !roots.iter().any(|r| r == &root) {
            roots.push(root);
        }
    }

    // Walk roots in parallel and match files against the GlobSet
    let collected: Vec<String> = roots
        .par_iter()
        .map(|root| {
            let mut out: Vec<String> = Vec::new();
            let walker = WalkBuilder::new(root)
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .hidden(false)
                .ignore(true)
                .parents(true)
                .build();

            for dent in walker {
                let dent = match dent {
                    Ok(d) => d,
                    Err(_e) => continue,
                };
                let ftype = match dent.file_type() {
                    Some(t) => t,
                    None => continue,
                };
                if !ftype.is_file() {
                    continue;
                }
                let s = dent.path().to_string_lossy();
                if set.is_match(s.as_ref()) {
                    out.push(s.to_string());
                }
            }
            out
        })
        .flatten()
        .collect();

    // If nothing matched, preserve original inputs to surface errors later
    if collected.is_empty() {
        return Ok(inputs.clone());
    }

    // Deduplicate while preserving order
    let mut seen: HashSet<String> = HashSet::new();
    let mut results: Vec<String> = Vec::with_capacity(collected.len());
    for s in collected {
        if seen.insert(s.clone()) {
            results.push(s);
        }
    }
    Ok(results)
}
