# View Status Filter Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend `langcodec view` to support status-based filtering with `--status`, `--keys-only`, and `--json`, including strict-mode safeguards for formats without explicit status metadata.

**Architecture:** Add new CLI flags on the existing `view` subcommand in `langcodec-cli/src/main.rs`, then route to an expanded view pipeline in `langcodec-cli/src/view.rs` that parses statuses, filters entries per language, and renders text/JSON outputs. Keep backward compatibility by preserving existing behavior when new flags are not used, and add strict-mode validation that only permits status filtering on explicit-status formats (`.xcstrings`) in v1.

**Tech Stack:** Rust 2024, `clap`, `serde_json`, existing `langcodec` status model (`EntryStatus`), `assert_cmd`, `tempfile`, Cargo workspace tests.

**Execution Skills:** @test-driven-development @verification-before-completion

---

### Task 1: Add `view` CLI surface for status filtering

**Files:**
- Modify: `langcodec-cli/src/main.rs`
- Modify: `langcodec-cli/src/view.rs`
- Test: `langcodec-cli/tests/cli_integration_tests.rs`

**Step 1: Write the failing test**

Add a new test to `langcodec-cli/tests/cli_integration_tests.rs`:

```rust
#[test]
fn test_view_help_lists_new_filter_flags() {
    let output = langcodec_cmd().args(["view", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--status"));
    assert!(stdout.contains("--keys-only"));
    assert!(stdout.contains("--json"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p langcodec-cli test_view_help_lists_new_filter_flags -- --nocapture`
Expected: FAIL because `view` does not expose these flags yet.

**Step 3: Write minimal implementation**

- In `langcodec-cli/src/main.rs`, extend `Commands::View` with:

```rust
#[arg(long)]
status: Option<String>,

#[arg(long, default_value_t = false)]
keys_only: bool,

#[arg(long, default_value_t = false)]
json: bool,
```

- Add a `ViewOptions` struct in `langcodec-cli/src/view.rs` and update `print_view` signature:

```rust
pub struct ViewOptions {
    pub full: bool,
    pub status: Option<String>,
    pub keys_only: bool,
    pub json: bool,
}

pub fn print_view(codec: &Codec, lang_filter: &Option<String>, opts: &ViewOptions) {
    // preserve current behavior for now
}
```

- Update the `Commands::View` branch in `main.rs` to construct and pass `ViewOptions`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p langcodec-cli test_view_help_lists_new_filter_flags -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec-cli/src/main.rs langcodec-cli/src/view.rs langcodec-cli/tests/cli_integration_tests.rs
git commit -m "feat(cli): add view filter flag surface"
```

### Task 2: Implement status parsing and entry filtering for text view

**Files:**
- Modify: `langcodec-cli/src/view.rs`
- Create: `langcodec-cli/tests/view_status_cli_tests.rs`

**Step 1: Write the failing tests**

Create `langcodec-cli/tests/view_status_cli_tests.rs` with initial status-filter tests:

```rust
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

#[test]
fn test_view_status_filters_single_status_for_lang() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("Localizable.xcstrings");

    let xc = r#"{
      "sourceLanguage": "en",
      "version": "1.0",
      "strings": {
        "welcome": {
          "localizations": {
            "fr": { "stringUnit": { "state": "new", "value": "" } }
          }
        },
        "bye": {
          "localizations": {
            "fr": { "stringUnit": { "state": "translated", "value": "Salut" } }
          }
        }
      }
    }"#;
    fs::write(&input_file, xc).unwrap();

    let output = langcodec_cmd()
        .args([
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--lang",
            "fr",
            "--status",
            "new",
            "--full",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("welcome"));
    assert!(!stdout.contains("bye"));
}

#[test]
fn test_view_status_rejects_invalid_status() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("Localizable.xcstrings");
    fs::write(
        &input_file,
        r#"{"sourceLanguage":"en","version":"1.0","strings":{}}"#,
    )
    .unwrap();

    let output = langcodec_cmd()
        .args([
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--status",
            "bad_status",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p langcodec-cli view_status_filters_single_status_for_lang view_status_rejects_invalid_status -- --nocapture`
Expected: FAIL because status parsing/filtering is not implemented.

**Step 3: Write minimal implementation**

In `langcodec-cli/src/view.rs`:

- Add parsed status utility:

```rust
fn parse_statuses(raw: &Option<String>) -> Result<Option<std::collections::HashSet<EntryStatus>>, String> {
    // split by comma, trim, normalize '-'/' ' to '_', parse EntryStatus
}
```

- Add filter predicate:

```rust
fn matches_status(entry: &langcodec::types::Entry, wanted: &Option<HashSet<EntryStatus>>) -> bool {
    wanted.as_ref().map(|set| set.contains(&entry.status)).unwrap_or(true)
}
```

- Filter entries before printing in text mode.
- Return a user-friendly error on invalid status token listing accepted statuses.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p langcodec-cli view_status_filters_single_status_for_lang view_status_rejects_invalid_status -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec-cli/src/view.rs langcodec-cli/tests/view_status_cli_tests.rs
git commit -m "feat(cli): add status filtering for view text output"
```

### Task 3: Add `--keys-only` text output and cross-language behavior

**Files:**
- Modify: `langcodec-cli/src/view.rs`
- Modify: `langcodec-cli/tests/view_status_cli_tests.rs`

**Step 1: Write the failing tests**

Add tests:

```rust
#[test]
fn test_view_keys_only_with_lang_prints_key_per_line() {
    // fixture with fr entries and mixed statuses
    // run: view --lang fr --status new,needs_review --keys-only
    // assert output lines contain only keys, no "Status:" lines
}

#[test]
fn test_view_keys_only_without_lang_prints_lang_tab_key() {
    // fixture where same key exists in en and fr and both match
    // run: view --status new --keys-only
    // assert lines include "en\t<key>" and "fr\t<key>"
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p langcodec-cli view_keys_only_ -- --nocapture`
Expected: FAIL because keys-only rendering is not implemented.

**Step 3: Write minimal implementation**

In `langcodec-cli/src/view.rs`:

- Branch text rendering when `opts.keys_only` is true.
- Emit:
  - `println!("{}", entry.id)` when `lang_filter.is_some()`
  - `println!("{}\t{}", resource.metadata.language, entry.id)` when `lang_filter.is_none()`
- Do not print verbose entry metadata in keys-only mode.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p langcodec-cli view_keys_only_ -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec-cli/src/view.rs langcodec-cli/tests/view_status_cli_tests.rs
git commit -m "feat(cli): add keys-only output for view filters"
```

### Task 4: Add JSON output for filtered entries and keys-only mode

**Files:**
- Modify: `langcodec-cli/src/view.rs`
- Modify: `langcodec-cli/tests/view_status_cli_tests.rs`

**Step 1: Write the failing tests**

Add JSON tests:

```rust
#[test]
fn test_view_status_json_outputs_entries_payload() {
    // run: view -i <xcstrings> --status new,needs_review --json
    // parse stdout as JSON
    // assert payload["summary"]["total_matches"] > 0
    // assert payload["entries"].is_array()
}

#[test]
fn test_view_status_json_keys_only_outputs_keys_payload() {
    // run: view -i <xcstrings> --status new --json --keys-only
    // parse JSON
    // assert payload has "summary" and "keys"
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p langcodec-cli view_status_json_ -- --nocapture`
Expected: FAIL because JSON rendering is not implemented for `view`.

**Step 3: Write minimal implementation**

In `langcodec-cli/src/view.rs`:

- Add serde-serializable payload structs:

```rust
#[derive(serde::Serialize)]
struct ViewSummary { total_matches: usize, languages: Vec<String>, statuses: Vec<String> }

#[derive(serde::Serialize)]
struct JsonEntry { lang: String, key: String, status: String, r#type: String, value: Option<String>, plural_forms: Option<std::collections::BTreeMap<String, String>>, comment: Option<String> }
```

- Render JSON using `serde_json::to_string_pretty`.
- For `--json --keys-only`, emit `summary` plus `keys` array (`{lang,key}` when no `--lang`).
- In JSON mode, always emit full values (ignore text truncation concerns).

**Step 4: Run tests to verify they pass**

Run: `cargo test -p langcodec-cli view_status_json_ -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec-cli/src/view.rs langcodec-cli/tests/view_status_cli_tests.rs
git commit -m "feat(cli): add json output for view status filters"
```

### Task 5: Enforce strict-mode status metadata guard

**Files:**
- Modify: `langcodec-cli/src/main.rs`
- Modify: `langcodec-cli/tests/view_status_cli_tests.rs`

**Step 1: Write the failing tests**

Add strict-mode tests:

```rust
#[test]
fn test_view_status_strict_fails_on_android_strings() {
    // create values/strings.xml
    // run: langcodec --strict view -i <xml> --status new
    // assert non-zero and stderr mentions explicit status metadata
}

#[test]
fn test_view_status_strict_allows_xcstrings() {
    // run: langcodec --strict view -i <xcstrings> --status new
    // assert success
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p langcodec-cli view_status_strict_ -- --nocapture`
Expected: FAIL because strict status-format gating is not implemented.

**Step 3: Write minimal implementation**

In `langcodec-cli/src/main.rs`:

- Add a helper:

```rust
fn supports_explicit_status_metadata(input: &str) -> bool {
    input.ends_with(".xcstrings")
}
```

- In `Commands::View`, before printing:

```rust
if strict && status.is_some() && !supports_explicit_status_metadata(&input) {
    eprintln!("❌ Strict status filtering requires explicit status metadata (supported in v1: .xcstrings)");
    std::process::exit(1);
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p langcodec-cli view_status_strict_ -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add langcodec-cli/src/main.rs langcodec-cli/tests/view_status_cli_tests.rs
git commit -m "fix(cli): enforce strict status filtering metadata requirement"
```

### Task 6: Update docs and run final verification

**Files:**
- Modify: `langcodec-cli/README.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Verify: `langcodec-cli/tests/view_status_cli_tests.rs` and existing suites

**Step 1: Write failing doc-oriented coverage check**

Add one integration test to ensure CLI help advertises final flags together:

```rust
#[test]
fn test_view_help_includes_status_examples_flags() {
    let output = langcodec_cmd().args(["view", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--status"));
    assert!(stdout.contains("--keys-only"));
    assert!(stdout.contains("--json"));
}
```

**Step 2: Run test to verify it passes before docs edit**

Run: `cargo test -p langcodec-cli test_view_help_includes_status_examples_flags -- --nocapture`
Expected: PASS.

**Step 3: Write documentation changes**

- Update `langcodec-cli/README.md` `view` section with new examples:
  - `langcodec view -i Localizable.xcstrings --status new,needs_review`
  - `langcodec view -i Localizable.xcstrings --status new --lang fr --json`
  - `langcodec view -i Localizable.xcstrings --status new,needs_review --keys-only`
- Update root `README.md` quick reference similarly.
- Add changelog line under unreleased/features.

**Step 4: Run final verification suite**

Run:
- `cargo test -p langcodec-cli view_status_ -- --nocapture`
- `cargo test -p langcodec-cli test_view_help_lists_new_filter_flags -- --nocapture`
- `cargo test -p langcodec-cli -- --nocapture`

Expected: PASS for all.

**Step 5: Commit**

```bash
git add langcodec-cli/README.md README.md CHANGELOG.md langcodec-cli/tests/cli_integration_tests.rs langcodec-cli/tests/view_status_cli_tests.rs
git commit -m "docs(cli): document view status filtering workflow"
```

## Final Verification Gate

Before merging:

- Run `cargo test --all`
- Run `cargo fmt --all -- --check`
- Run `cargo clippy --all-targets --all-features -- -D warnings`

If any command fails, fix issues and re-run until clean.
