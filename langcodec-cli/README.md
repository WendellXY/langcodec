# langcodec-cli (Command Line)

Universal CLI for converting, inspecting, merging, and editing localization files.

- Formats: Apple `.strings`, `.xcstrings`, Android `strings.xml`, CSV, TSV
- Commands: convert, merge, view, stats, debug, edit

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

Supported formats: .strings, .xml (Android), .xcstrings, .csv, .tsv. Custom JSON/YAML/.langcodec edit is currently not enabled.

## Notes

- Android plurals `<plurals>` are supported.
- Language inference: `en.lproj/Localizable.strings`, `values-es/strings.xml`, base `values/` → `en` by default.
- Globbing: use quotes for patterns in merge and edit (e.g., `'**/*.xml'`).

## License

MIT
