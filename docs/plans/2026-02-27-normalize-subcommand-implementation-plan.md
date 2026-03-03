# Normalize Subcommand Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `normalize` subcommand that safely canonicalizes localization files across standard formats with in-place writes by default and CI-friendly `--check` drift detection.

**Architecture:** Implement a shared normalization engine in `langcodec` that applies deterministic rules (ordering, formatting-compatible canonicalization, optional placeholders, optional key-style transform with collision detection) and returns change metadata. Wire `langcodec-cli` `normalize` to this engine for glob expansion, validation, write/check behavior, and summary reporting.

**Tech Stack:** Rust 2024, `clap`, `langcodec` parser/writer APIs, `assert_cmd`, `tempfile`, workspace Cargo tests.

---

### Task 1: Add `normalize` CLI surface (scaffold only)

**Files:**
- Create: `langcodec-cli/src/normalize.rs`
- Modify: `langcodec-cli/src/main.rs`
- Test: `langcodec-cli/tests/normalize_cli_tests.rs`

**Step 1: Write the failing test**

```rust
use std::process::Command;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

#[test]
fn test_main_help_lists_normalize() {
    let output = langcodec_cmd().args(["--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("normalize"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p langcodec-cli test_main_help_lists_normalize -- --nocapture`
Expected: FAIL because `normalize` subcommand is not registered.

**Step 3: Write minimal implementation**

```rust
// langcodec-cli/src/normalize.rs
pub fn run_normalize_command() -> Result<(), String> {
    Ok(())
}
```

```rust
// main.rs (minimal wiring)
mod normalize;
use crate::normalize::run_normalize_command;

// Add Commands::Normalize { ... } variant and branch to call run_normalize_command()
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p langcodec-cli test_main_help_lists_normalize -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec-cli/src/main.rs langcodec-cli/src/normalize.rs langcodec-cli/tests/normalize_cli_tests.rs
git commit -m "feat(cli): add normalize subcommand scaffold"
```

### Task 2: Add failing library tests for deterministic normalization core

**Files:**
- Create: `langcodec/src/normalize.rs`
- Modify: `langcodec/src/lib.rs`
- Test: `langcodec/tests/normalize_engine_tests.rs`

**Step 1: Write the failing test**

```rust
use langcodec::{Codec, types::{Entry, EntryStatus, Metadata, Resource, Translation}};
use std::collections::HashMap;

#[test]
fn normalize_sorts_entries_and_is_idempotent() {
    let mut codec = Codec {
        resources: vec![Resource {
            metadata: Metadata { language: "en".into(), domain: "Localizable".into(), custom: HashMap::new() },
            entries: vec![
                Entry { id: "z_key".into(), value: Translation::Singular("Z".into()), comment: None, status: EntryStatus::Translated, custom: HashMap::new() },
                Entry { id: "a_key".into(), value: Translation::Singular("A".into()), comment: None, status: EntryStatus::Translated, custom: HashMap::new() },
            ],
        }],
    };

    let report1 = langcodec::normalize::normalize_codec(&mut codec, &Default::default()).unwrap();
    let ids: Vec<_> = codec.resources[0].entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["a_key", "z_key"]);
    assert!(report1.changed);

    let report2 = langcodec::normalize::normalize_codec(&mut codec, &Default::default()).unwrap();
    assert!(!report2.changed);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p langcodec normalize_sorts_entries_and_is_idempotent -- --nocapture`
Expected: FAIL because `normalize` module/API does not exist.

**Step 3: Write minimal implementation**

```rust
// langcodec/src/normalize.rs
#[derive(Debug, Clone, Default)]
pub struct NormalizeOptions {
    pub normalize_placeholders: bool,
    pub key_style: KeyStyle,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum KeyStyle { #[default] None, Snake, Kebab, Camel }

#[derive(Debug, Clone, Default)]
pub struct NormalizeReport { pub changed: bool }

pub fn normalize_codec(codec: &mut crate::Codec, _opts: &NormalizeOptions) -> Result<NormalizeReport, crate::Error> {
    let before = codec.clone();
    for resource in &mut codec.resources {
        resource.entries.sort_by(|a, b| a.id.cmp(&b.id));
    }
    Ok(NormalizeReport { changed: *codec != before })
}
```

```rust
// lib.rs
pub mod normalize;
pub use crate::normalize::{normalize_codec, KeyStyle, NormalizeOptions, NormalizeReport};
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p langcodec normalize_sorts_entries_and_is_idempotent -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec/src/normalize.rs langcodec/src/lib.rs langcodec/tests/normalize_engine_tests.rs
git commit -m "feat(core): add normalization engine skeleton"
```

### Task 3: Add placeholder toggle and key-style collision safety

**Files:**
- Modify: `langcodec/src/normalize.rs`
- Test: `langcodec/tests/normalize_engine_tests.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn normalize_applies_placeholders_by_default() {
    let mut codec = fixture_codec_with_value("welcome", "%@ has %ld items");
    let report = langcodec::normalize::normalize_codec(&mut codec, &Default::default()).unwrap();
    let val = codec.find_entry("welcome", "en").unwrap().value.plain_translation_string();
    assert_eq!(val, "%s has %d items");
    assert!(report.changed);
}

#[test]
fn normalize_key_style_collision_fails() {
    let mut codec = fixture_codec_with_keys(&["welcome-title", "welcome_title"]);
    let opts = langcodec::normalize::NormalizeOptions {
        normalize_placeholders: true,
        key_style: langcodec::normalize::KeyStyle::Snake,
    };
    let err = langcodec::normalize::normalize_codec(&mut codec, &opts).unwrap_err();
    assert!(err.to_string().contains("collision"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p langcodec normalize_ -- --nocapture`
Expected: FAIL for missing placeholder/key-style behaviors.

**Step 3: Write minimal implementation**

```rust
// normalize.rs additions
if opts.normalize_placeholders {
    codec.normalize_placeholders_in_place();
}

// apply key transform when key_style != None
// build old->new mapping, detect duplicate new keys, return policy_violation error on collision
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p langcodec normalize_ -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec/src/normalize.rs langcodec/tests/normalize_engine_tests.rs
git commit -m "feat(core): add placeholder and key-style normalization rules"
```

### Task 4: Implement single-file CLI normalize behavior (`in-place`, `--output`, `--check`, `--dry-run`)

**Files:**
- Modify: `langcodec-cli/src/main.rs`
- Modify: `langcodec-cli/src/normalize.rs`
- Test: `langcodec-cli/tests/normalize_cli_tests.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_normalize_check_fails_on_drift() {
    let t = tempfile::TempDir::new().unwrap();
    let input = t.path().join("en.strings");
    std::fs::write(&input, "\"z\" = \"%@\";\n\"a\" = \"A\";\n").unwrap();

    let out = langcodec_cmd()
        .args(["normalize", "-i", input.to_str().unwrap(), "--check"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("would change"));
}

#[test]
fn test_normalize_dry_run_does_not_write() {
    let t = tempfile::TempDir::new().unwrap();
    let input = t.path().join("en.strings");
    let before = "\"z\" = \"%@\";\n\"a\" = \"A\";\n";
    std::fs::write(&input, before).unwrap();

    let out = langcodec_cmd()
        .args(["normalize", "-i", input.to_str().unwrap(), "--dry-run"])
        .output()
        .unwrap();

    assert!(out.status.success());
    let after = std::fs::read_to_string(&input).unwrap();
    assert_eq!(after, before);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p langcodec-cli test_normalize_ -- --nocapture`
Expected: FAIL because normalize behavior is not implemented.

**Step 3: Write minimal implementation**

```rust
// normalize.rs
#[derive(Debug, Clone)]
pub struct NormalizeOptions {
    pub inputs: Vec<String>,
    pub lang: Option<String>,
    pub output: Option<String>,
    pub dry_run: bool,
    pub check: bool,
    pub continue_on_error: bool,
    pub no_placeholders: bool,
    pub key_style: String,
}

// flow:
// 1) expand inputs with path_glob::expand_input_globs
// 2) read file into bytes + Codec
// 3) run langcodec::normalize::normalize_codec
// 4) serialize to temp bytes, compare against original
// 5) enforce --check / --dry-run / write rules
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p langcodec-cli test_normalize_ -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec-cli/src/main.rs langcodec-cli/src/normalize.rs langcodec-cli/tests/normalize_cli_tests.rs
git commit -m "feat(cli): implement normalize check and dry-run semantics"
```

### Task 5: Implement multi-file behavior, `--continue-on-error`, and `--output` constraints

**Files:**
- Modify: `langcodec-cli/src/normalize.rs`
- Test: `langcodec-cli/tests/normalize_cli_tests.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_normalize_rejects_output_with_multiple_inputs() {
    let t = tempfile::TempDir::new().unwrap();
    let a = t.path().join("a.strings");
    let b = t.path().join("b.strings");
    let out_file = t.path().join("out.strings");
    std::fs::write(&a, "\"z\" = \"Z\";\n").unwrap();
    std::fs::write(&b, "\"a\" = \"A\";\n").unwrap();

    let out = langcodec_cmd()
        .args([
            "normalize", "-i", a.to_str().unwrap(), "-i", b.to_str().unwrap(),
            "-o", out_file.to_str().unwrap()
        ])
        .output()
        .unwrap();

    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--output cannot be used with multiple"));
}

#[test]
fn test_normalize_continue_on_error_processes_remaining_files() {
    // one valid file + one missing literal path
    // expect non-zero, summary includes processed successes and failures
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p langcodec-cli normalize_rejects_output_with_multiple_inputs -- --nocapture`
Expected: FAIL for missing constraints.

**Step 3: Write minimal implementation**

```rust
// normalize.rs
// mirror edit.rs pattern for:
// - missing literal file detection
// - continue_on_error aggregation
// - summary: processed/success/failed/changed
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p langcodec-cli test_normalize_ -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec-cli/src/normalize.rs langcodec-cli/tests/normalize_cli_tests.rs
git commit -m "feat(cli): add normalize multi-file safety and error aggregation"
```

### Task 6: Cover all standard formats and final regression sweep

**Files:**
- Modify: `langcodec-cli/tests/normalize_cli_tests.rs`
- Modify: `langcodec/tests/normalize_engine_tests.rs`
- Modify: `langcodec-cli/README.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`

**Step 1: Write failing format coverage tests**

```rust
#[test]
fn test_normalize_supports_strings_xcstrings_android_csv_tsv() {
    // create one fixture per format with intentionally unsorted keys / placeholder variance
    // run normalize --check and assert drift for each
}
```

**Step 2: Run tests to verify failures**

Run: `cargo test -p langcodec-cli normalize_supports_ -- --nocapture`
Expected: FAIL until all format pathways are fully wired.

**Step 3: Implement minimal fixes and docs**

```markdown
# README snippets
- Add `normalize` command under CLI highlights.
- Document `--check`, `--no-placeholders`, and `--key-style`.
```

**Step 4: Run full verification**

Run: `cargo test -p langcodec`
Expected: PASS.

Run: `cargo test -p langcodec-cli`
Expected: PASS.

Run: `cargo test --all`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec/tests/normalize_engine_tests.rs langcodec-cli/tests/normalize_cli_tests.rs langcodec-cli/README.md README.md CHANGELOG.md
git commit -m "test(cli): add normalize format coverage and documentation"
```

## Notes for Executor
- Keep all normalization logic centralized in `langcodec/src/normalize.rs` to avoid CLI behavior drift.
- Preserve existing semantics for `view --check-plurals` exit code `2`; `normalize --check` drift should use exit `1`.
- Do not add support for custom one-way transformer inputs in v1 normalize.
- Keep key renaming opt-in only.
