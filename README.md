# langcodec

Universal localization toolkit: library + CLI for Apple/Android/CSV/TSV.

- Library crate (`langcodec`): parse, write, convert, merge with a unified model
- CLI crate (`langcodec-cli`): convert, diff, merge, sync, view, stats, debug, edit

---

## Status

This is a `0.6.4` release available on [crates.io](https://crates.io/crates/langcodec). As a 0.x version, APIs may evolve. Contributions and feedback are very welcome!

---

## Installation

- CLI: `cargo install langcodec-cli`
- Lib: add `langcodec = "0.6.4"` to your `Cargo.toml`

---

## Features

- Parse, write, convert, merge: `.strings`, `.xcstrings`, `strings.xml`, CSV, TSV
- Unified `Resource` model (`Translation::Singular|Plural`)
- Plurals: `.xcstrings` and Android `<plurals>` supported
- CLI helpers: convert, merge, view, stats (JSON or human-readable)

---

## Supported Formats

<!-- markdownlint-disable no-inline-html no-space-in-emphasis -->

| Format                | Parse | Write | Convert | Merge | Plural Support   | Comments |
|-----------------------|:-----:|:-----:|:-------:|:-----:|:----------------:|----------|
| Apple `.strings`      |  ✔️   |  ✔️   |   ✔️    |  ✔️   |   No             |  ✔️      |
| Apple `.xcstrings`    |  ✔️   |  ✔️   |   ✔️    |  ✔️   |   Yes<sup>*</sup>|  ✔️      |
| Android `strings.xml` |  ✔️   |  ✔️   |   ✔️    |  ✔️   |   Yes            |  ✔️      |
| CSV                   |  ✔️   |  ✔️   |   ✔️    |  ✔️   |   No             |  –       |
| TSV                   |  ✔️   |  ✔️   |   ✔️    |  ✔️   |   No             |  –       |

<sup>* `.xcstrings` plural support is implemented via CLDR categories.</sup>

<!-- markdownlint-enable no-inline-html no-space-in-emphasis -->

---

## Getting Started

- Library guide: see `langcodec/README.md`
- CLI guide: see `langcodec-cli/README.md`

---

### CLI Highlights

- Convert: `langcodec convert -i input.strings -o strings.xml`
- Diff: `langcodec diff --source A.xcstrings --target B.xcstrings --json`
- Edit (add/update/remove): `langcodec edit set -i 'locales/**/*.strings' -k welcome -v "Hello"` (use `--dry-run` to preview)
- Sync existing keys only: `langcodec sync --source A.xcstrings --target B.xcstrings --match-lang en`
- View: `langcodec view -i strings.xml --full`
- Stats (JSON): `langcodec stats -i Localizable.xcstrings --json`
  - See full options: langcodec-cli/README.md#stats
  - Example output:
  
    ```json
    {
      "summary": { "languages": 1, "unique_keys": 42 },
      "languages": [
        {
          "language": "en",
          "total": 42,
          "by_status": {
            "translated": 30,
            "needs_review": 2,
            "stale": 0,
            "new": 10,
            "do_not_translate": 0
          },
          "completion_percent": 75.0
        }
      ]
    }
    ```

#### Notes

- For CSV/TSV single-language files, the language code (`--lang`) may be required.
- All commands support Apple `.strings`, `.xcstrings`, Android `strings.xml`, CSV, and TSV.
- The convert command also supports custom JSON/YAML input formats.
- The CLI will error if you try to merge files of different formats.
- Edit supports multiple inputs and glob patterns. When multiple inputs are provided, edits are applied in-place and `--output` is not allowed.
- Android path inference: `values/strings.xml` (no qualifier) defaults to English (`en`).
- When converting to `.xcstrings`, if `source_language` or `version` metadata is missing, the CLI defaults them to `en` and `1.0` respectively (overridable via flags).

#### Plurals

- Android `<plurals>` are fully supported. They convert to the internal `Translation::Plural` representation and back to `<plurals>` with quantities `zero/one/two/few/many/other`.
- `.xcstrings` plural variations convert to Android `<plurals>` when targeting Android output.
- The `view` command prints plural entries with a "Type: Plural" header and each category/value.

For JSON/YAML custom formats and more examples, see `langcodec-cli/README.md`.

---

## Data Model

At the core is the `Resource` struct with `Entry` values (singular or plural). See `langcodec/README.md` and docs.rs for details.

---

## Roadmap & Contributing

- Roadmap: see `ROADMAP.md`
- Contributions welcome! Please open issues/PRs.

---

## Extending

Adding a new localization format?
Implement the `Parser` trait for your format struct in `formats/`, and add `From`/`TryFrom` conversions to and from `Resource`.
PRs welcome!

---

## Test Data

Sample test files for all supported formats are located in `tests/data/lib/` and `tests/data/cli/` at the workspace root. Use these for development, testing, and examples.

---

## Contributing

Contributions are welcome!
Please open issues for bugs, suggestions, or new format support.
See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

This project is licensed under the MIT License.

---

## Acknowledgements

- Inspired by the need for universal localization tooling in cross-platform apps
- Built with love in Rust

---

## Status and Roadmap

`langcodec` is now available on [crates.io](https://crates.io/crates/langcodec). As a 0.x version, the API may evolve as development continues. The current focus is on expanding format support, improving the CLI experience, and building a robust ecosystem for localization tooling. We welcome your issues, feature requests, and discussions at the project's issue tracker.
