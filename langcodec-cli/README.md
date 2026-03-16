# langcodec-cli

`langcodec` is a command-line tool for real localization work.

It helps you move between Apple, Android, and tabular translation formats without building one-off scripts for every project.

Supported inputs and outputs:

- Apple `.strings`
- Apple `.xcstrings`
- Android `strings.xml`
- CSV
- TSV

## Why Use It

`langcodec` is designed for teams who need to:

- convert localization files across platforms
- inspect missing, stale, or review-needed strings
- normalize files to reduce noisy diffs
- edit translations in place
- merge or sync catalogs safely
- draft translations with AI providers

Instead of treating localization as a pile of ad hoc file conversions, `langcodec` gives you one CLI that works across common formats and workflows.

## Install

```sh
cargo install langcodec-cli
```

## Start Here

The CLI should teach the detailed usage directly:

```sh
langcodec --help
langcodec convert --help
langcodec translate --help
langcodec view --help
```

This README is intentionally brief. Use it to understand what the tool is good at, then use built-in help for exact flags and behavior.

## Core Workflows

### Convert between ecosystems

```sh
langcodec convert -i Localizable.xcstrings -o translations.csv
langcodec convert -i translations.csv -o values/strings.xml
```

### Find untranslated or review-needed strings

```sh
langcodec view -i Localizable.xcstrings --status new,needs_review --keys-only
langcodec stats -i Localizable.xcstrings --json
```

### Edit files without format-specific tooling

```sh
langcodec edit set -i en.strings -k welcome_title -v "Welcome"
langcodec edit set -i values/strings.xml -k welcome_title -v "Welcome"
```

### Normalize files for cleaner diffs

```sh
langcodec normalize -i 'locales/**/*.{strings,xml,csv,tsv,xcstrings}' --check
```

### Sync or merge existing translation assets

```sh
langcodec sync --source source.xcstrings --target target.xcstrings --match-lang en
langcodec merge -i a.xcstrings -i b.xcstrings -o merged.xcstrings --strategy last
```

### Draft translations with AI

```sh
langcodec translate \
  --source Localizable.xcstrings \
  --source-lang en \
  --target-lang fr,de,ja \
  --provider openai \
  --model gpt-4.1-mini
```

`translate` supports:

- in-place updates for multi-language files like `.xcstrings`
- config defaults from `langcodec.toml`
- multiple target languages for multi-language outputs
- live progress updates
- preflight validation before model requests
- translation result summaries at the end

## Example Config

```toml
[translate]
source = "locales/Localizable.xcstrings"
provider = "openai"
model = "gpt-4.1-mini"
source_lang = "en"
target_lang = "fr,de"
status = ["new", "stale"]
concurrency = 4
```

Then run:

```sh
langcodec translate
```

For larger repos, `translate.sources = [...]` can fan out parallel runs from config.

## Main Commands

- `convert`: convert between localization formats
- `view`: inspect entries, statuses, and keys
- `stats`: summarize coverage and completion
- `edit`: add, update, or remove entries
- `normalize`: rewrite files into a stable form
- `diff`: compare two localization files
- `sync`: update existing target entries from a source file
- `merge`: combine multiple inputs into one output
- `translate`: draft translations with AI-backed providers
- `debug`: inspect parsed output as JSON

## When It Fits Best

`langcodec` is especially useful if you are:

- maintaining both iOS and Android apps
- passing strings through translators or spreadsheets
- trying to reduce localization-related CI drift
- replacing fragile custom scripts with one reusable tool

## Related Docs

- Root overview: [README.md](../README.md)
- Rust library crate: [langcodec/README.md](../langcodec/README.md)

## License

MIT
