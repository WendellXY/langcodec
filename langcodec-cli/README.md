# langcodec-cli (Command Line)

Universal CLI for converting, inspecting, and merging localization files.

- Formats: Apple `.strings`, `.xcstrings`, Android `strings.xml`, CSV, TSV
- Commands: convert, merge, view, stats, debug

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

## Notes

- Android plurals `<plurals>` are supported.
- Language inference: `en.lproj/Localizable.strings`, `values-es/strings.xml`, base `values/` → `en` by default.
- Globbing: use quotes for patterns in merge (e.g., `'**/*.xml'`).

## License

MIT
