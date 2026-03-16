# langcodec

`langcodec` is a Rust toolkit for working with localization files across Apple, Android, and spreadsheet-style workflows.

It gives you one consistent model for parsing, converting, inspecting, editing, merging, normalizing, and translating files like:

- Apple `.strings`
- Apple `.xcstrings`
- Android `strings.xml`
- CSV
- TSV

This workspace includes:

- `langcodec`: the Rust library crate
- `langcodec-cli`: the `langcodec` command-line tool

## Why People Use It

Localization pipelines usually get messy when teams have to move between iOS, Android, translators, spreadsheets, and CI scripts. `langcodec` is built to reduce that friction.

With one toolchain, you can:

- convert catalogs between Apple, Android, CSV, and TSV
- inspect missing or stale entries
- merge and sync translations safely
- edit files in place across formats
- normalize files to reduce noisy diffs
- generate draft translations with AI-backed providers
- generate translator-facing xcstrings comments from source usage

## What It Feels Like

```sh
# Convert Apple strings to Android XML
langcodec convert -i Localizable.strings -o strings.xml

# Inspect untranslated entries
langcodec view -i Localizable.xcstrings --status new,needs_review --keys-only

# Normalize localization files in CI
langcodec normalize -i 'locales/**/*.{strings,xml,csv,tsv,xcstrings}' --check

# Draft translations into an .xcstrings catalog
langcodec translate \
  --source Localizable.xcstrings \
  --source-lang en \
  --target-lang fr,de,ja \
  --provider openai \
  --model gpt-4.1-mini

# Generate xcstrings comments with source-aware AI annotation
langcodec annotate \
  --input Localizable.xcstrings \
  --source-root Sources \
  --source-root Modules \
  --provider openai \
  --model gpt-4.1-mini
```

## Highlights

- Unified data model for singular and plural translations
- Read/write support for Apple, Android, CSV, and TSV formats
- CLI commands for convert, diff, merge, sync, edit, normalize, view, stats, debug, translate, and annotate
- `.xcstrings` and Android plural support
- Config-driven translate and annotate workflows with `langcodec.toml`
- Rust library API for building your own tooling on top

## Installation

Install the CLI:

```sh
cargo install langcodec-cli
```

Use the library in Rust:

```toml
[dependencies]
langcodec = "0.9.1"
```

## Supported Formats

| Format                | Parse | Write | Convert | Merge | Plurals | Comments |
| --------------------- | :---: | :---: | :-----: | :---: | :-----: | :------: |
| Apple `.strings`      |  yes  |  yes  |   yes   |  yes  |   no    |   yes    |
| Apple `.xcstrings`    |  yes  |  yes  |   yes   |  yes  |   yes   |   yes    |
| Android `strings.xml` |  yes  |  yes  |   yes   |  yes  |   yes   |   yes    |
| CSV                   |  yes  |  yes  |   yes   |  yes  |   no    |    no    |
| TSV                   |  yes  |  yes  |   yes   |  yes  |   no    |    no    |

## CLI Quick Start

### Convert files

```sh
langcodec convert -i input.xcstrings -o output.csv
langcodec convert -i input.csv -o output.xcstrings --source-language en --version 1.0
```

### Inspect work to do

```sh
langcodec view -i Localizable.xcstrings --status new,needs_review
langcodec stats -i Localizable.xcstrings --json
```

### Edit and normalize

```sh
langcodec edit set -i en.strings -k welcome_title -v "Welcome"
langcodec normalize -i values/strings.xml
```

### Merge and sync

```sh
langcodec merge -i a.xcstrings -i b.xcstrings -o merged.xcstrings --strategy last
langcodec sync --source source.xcstrings --target target.xcstrings --match-lang en
```

### AI workflows with config

Create a `langcodec.toml` in your project:

```toml
[ai]
provider = "openai"
model = "gpt-4.1-mini"

[translate]
source = "locales/Localizable.xcstrings"
source_lang = "en"
target_lang = "fr,de"
status = ["new", "stale"]
concurrency = 4

[annotate]
input = "locales/Localizable.xcstrings"
source_roots = ["Sources", "Modules"]
concurrency = 4
```

Then run:

```sh
langcodec translate
langcodec annotate
```

`translate` still accepts legacy `translate.provider` and `translate.model` if you have older config files. For larger projects, `translate.sources = [...]` can fan out parallel runs from config.

`annotate` also supports `annotate.inputs = [...]` for config-driven in-place runs across multiple xcstrings files.

More CLI details live in [langcodec-cli/README.md](langcodec-cli/README.md).

## Library Quick Start

```rust
use langcodec::{Codec, convert_auto};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    convert_auto("Localizable.strings", "strings.xml")?;

    let mut codec = Codec::new();
    codec.read_file_by_extension("Localizable.xcstrings", None)?;

    for language in codec.languages() {
        println!("{language}");
    }

    Ok(())
}
```

The library is a good fit if you want to:

- build custom localization pipelines in Rust
- validate translation assets in CI
- write converters or format-specific tooling
- work with a common representation instead of format-specific parsing code

More library details live in [langcodec/README.md](langcodec/README.md).

## Project Layout

- [langcodec](langcodec): Rust library crate
- [langcodec-cli](langcodec-cli): command-line interface
- [tests](tests): shared test data and integration coverage

## Current Status

The current release is `0.9.1` on [crates.io](https://crates.io/crates/langcodec). It is already useful in real workflows, but it is still a `0.x` project, so APIs and behavior may continue to evolve.

## Contributing

Issues, ideas, and pull requests are welcome.

- Project roadmap: [ROADMAP.md](ROADMAP.md)
- Contribution guide: [CONTRIBUTING.md](CONTRIBUTING.md)

## License

MIT
