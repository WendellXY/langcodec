# langcodec-cli (Command Line)

Universal CLI for converting, inspecting, merging, and editing localization files.

- Formats: Apple `.strings`, `.xcstrings`, Android `strings.xml`, CSV, TSV
- Commands: convert, diff, merge, sync, view, stats, debug, edit, normalize

## Install

```sh
cargo install langcodec-cli
```

## Commands

### convert

```sh
langcodec convert -i input.strings -o strings.xml
langcodec convert -i input.csv -o output.xcstrings --source-language en --version 1.0
```

Auto-detects formats from extensions. For JSON/YAML custom formats, see `--input-format` in the root README.

### merge

```sh
langcodec merge -i a.strings -i b.strings -o merged.strings --lang en --strategy last
```

### diff

Compare source file A against target file B.

```sh
langcodec diff --source A.xcstrings --target B.xcstrings
langcodec diff --source A.csv --target B.csv --json --output diff_report.json
```

Outputs added/removed/changed keys by language.

### sync

Sync values from source file A into existing keys in target file B.
This command updates only keys that already exist in target.

```sh
langcodec sync --source A.xcstrings --target B.xcstrings --match-lang en
langcodec sync --source source.csv --target target.csv --output synced.csv --match-lang en
```

Matching rules:
- key-to-key match first
- fallback: use `--match-lang` translation (default inferred/en) to match source entries
- never adds new keys to target

CI-oriented options:
- `--report-json <path>` write sync summary as JSON
- `--fail-on-unmatched` return non-zero when unmatched entries exist
- `--fail-on-ambiguous` return non-zero when fallback matching is ambiguous

### view

```sh
langcodec view -i values/strings.xml --full
langcodec view -i Localizable.xcstrings --status new,needs_review
langcodec view -i Localizable.xcstrings --status new --lang fr --json
langcodec view -i Localizable.xcstrings --status new,needs_review --keys-only
```

Prints entries. Plurals are labeled with `Type: Plural` and show categories.

View options:

- `--status`: Filter by one or more statuses (`translated|needs_review|new|do_not_translate|stale`), comma-separated.
- `--keys-only`: Print only keys in text mode (`lang<TAB>key` when `--lang` is not set).
- `--json`: Output machine-readable JSON (`summary` + `entries` or `keys` payload).
- `--lang`: Restrict results to a specific language before status filtering.
- `--strict`: With `--status`, requires explicit status metadata (supported in v1: `.xcstrings`).

### stats

```sh
langcodec stats -i values/strings.xml
langcodec stats -i Localizable.xcstrings --json
```

Shows per-language totals, counts by status, and completion percent (excludes DoNotTranslate). Use `--json` for machine-readable output.

### debug

```sh
langcodec debug -i input.strings --lang en -o output.json
```

### edit

Unified in-place editing (add/update/remove) across one or many files.

Basics:

```sh
# Add or update a key
langcodec edit set -i en.strings -k welcome -v "Hello" --status translated --comment "Shown on home"

# Remove a key (omit or empty value)
langcodec edit set -i en.strings -k welcome
langcodec edit set -i en.strings -k welcome -v ""

# Multiple files or globs (quote patterns)
langcodec edit set -i 'locales/**/*.strings' -k app_name -v "My App"
langcodec edit set -i a.strings -i b.strings -k welcome -v "Hello"

# Preview only
langcodec edit set -i en.strings -k welcome -v "Hello" --dry-run

# Write to a different file (single input only)
langcodec edit set -i en.strings -k welcome -v "Hello" -o out.strings
```

Options:

- --inputs/-i: One or more input files. Supports glob patterns when quoted.
- --lang/-l: Language code (required when an input contains multiple languages).
- --key/-k: Entry key to modify.
- --value/-v: New value. If omitted or empty, the entry is removed.
- --comment: Optional translator note.
- --status: translated|needs_review|new|do_not_translate|stale.
- --output/-o: Optional output path. Not allowed with multiple inputs.
- --dry-run: Print what would change and exit without writing.
- --continue-on-error: Process all inputs; report failures at the end (non-zero exit if any fail).

Supported formats: .strings, .xml (Android), .xcstrings, .csv, .tsv. Custom JSON/YAML/.langcodec edit is currently not enabled.

### normalize

Normalize localization files in-place (or to `--output` in single-input mode).

```sh
# Normalize in-place
langcodec normalize -i en.lproj/Localizable.strings

# CI drift check (non-zero if any file would change)
langcodec normalize -i 'locales/**/*.{strings,xml,csv,tsv,xcstrings}' --check

# Preview without writing
langcodec normalize -i values/strings.xml --dry-run

# Disable placeholder normalization and rename keys to snake_case
langcodec normalize -i Localizable.xcstrings --no-placeholders --key-style snake

# Keep processing remaining files and summarize failures at the end
langcodec normalize -i a.strings -i b.csv -i c.tsv --continue-on-error
```

Options and behavior:

- --check: Detects normalization drift and exits non-zero when a file would change.
- --dry-run: Prints what would change and exits without writing files.
- --no-placeholders: Skips placeholder canonicalization (for example `%@` → `%s`).
- --key-style: Renames keys during normalization. Values: `none` (default), `snake`, `kebab`, `camel`.
- --output/-o: Single-input mode only. If multiple inputs are provided, `--output` is rejected.
- --continue-on-error: Continues processing all matched inputs, prints a summary, and exits non-zero if any file failed.
- --inputs/-i: One or more files, including quoted glob patterns.

Supported normalize formats: `.strings`, Android `strings.xml`, `.csv`, `.tsv`, `.xcstrings`.

## Notes

- Android plurals `<plurals>` are supported.
- Language inference: `en.lproj/Localizable.strings`, `values-es/strings.xml`, base `values/` → `en` by default.
- Globbing: use quotes for patterns in merge and edit (e.g., `'**/*.xml'`).
- Global strict mode: add `--strict` before any subcommand to disable parser fallbacks and enforce stricter failures.

## License

MIT
