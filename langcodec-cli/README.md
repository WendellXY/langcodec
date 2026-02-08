# langcodec-cli (Command Line)

Universal CLI for converting, inspecting, merging, and editing localization files.

- Formats: Apple `.strings`, `.xcstrings`, Android `strings.xml`, CSV, TSV
- Commands: convert, diff, merge, sync, view, stats, debug, edit

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
```

Prints entries. Plurals are labeled with `Type: Plural` and show categories.

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

## Notes

- Android plurals `<plurals>` are supported.
- Language inference: `en.lproj/Localizable.strings`, `values-es/strings.xml`, base `values/` → `en` by default.
- Globbing: use quotes for patterns in merge and edit (e.g., `'**/*.xml'`).
- Global strict mode: add `--strict` before any subcommand to disable parser fallbacks and enforce stricter failures.

## License

MIT
