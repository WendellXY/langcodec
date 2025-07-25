# langcodec

**A universal localization file toolkit in Rust.**

`langcodec` provides format-agnostic parsing, conversion, merging, and serialization for major localization formats, including Apple `.strings`, `.xcstrings`, Android `strings.xml`, and CSV. It enables seamless conversion and merging between formats, powerful internal data modeling, and extensibility for new formats.

---

## Status

This is an early `0.1.0` release. The API may evolve as development continues. Contributions and feedback are very welcome to help shape the future of this project!

---

## Features

- ✨ Parse, write, convert, and merge multiple localization file formats
- 🦀 Idiomatic, modular, and ergonomic Rust API
- 📦 Designed for CLI tools, CI/CD pipelines, and library integration
- 🔄 Unified internal model (`Resource`) for lossless format-agnostic processing
- 📖 Well-documented, robust error handling and extensible codebase
- 🚀 More formats and CLI support are planned for upcoming releases

---

## Supported Formats

<!-- markdownlint-disable no-inline-html no-space-in-emphasis -->

| Format                | Parse | Write | Convert | Merge | Plural Support   | Comments |
|-----------------------|:-----:|:-----:|:-------:|:-----:|:----------------:|----------|
| Apple `.strings`      |  ✔️   |  ✔️   |   ✔️    |  ✔️   |   No             |  ✔️      |
| Apple `.xcstrings`    |  ✔️   |  ✔️   |   ✔️    |  ✔️   |   Yes<sup>*</sup>|  ✔️      |
| Android `strings.xml` |  ✔️   |  ✔️   |   ✔️    |  ✔️   |   No<sup>*</sup> |  ✔️      |
| CSV                   |  ✔️   |  ✔️   |   ✔️    |  ✔️   |   No             |  –       |

<sup>* Plural support for `.xcstrings` is not under beta testing, and may not be fully implemented yet.</sup>
<sup>* Plural support for Android may be added in the future.</sup>

<!-- markdownlint-enable no-inline-html no-space-in-emphasis -->

---

## Usage

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
langcodec = "0.1"
```

#### Example: Read, Manipulate, and Write

```rust
use langcodec::{Codec, formats::FormatType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut codec = Codec::new();

    // Read Apple .strings file
    codec.read_file_by_extension("en.lproj/Localizable.strings", None)?;

    // Manipulate resources if needed (see types.rs for Resource/Entry APIs)

    // Write changes back to the original file
    codec.write_to_file()?;

    // Convert Apple's strings localization to Android's strings
    convert_auto("Localizable.strings", "strings.xml")?;

    Ok(())
}
```

---

### CLI

A CLI tool is provided for easy conversion, merging, and debugging of localization files.

#### Install (from source)

```sh
cargo install --path langcodec-cli
```

#### Commands

- **Convert** between formats:

  ```sh
  langcodec convert -i input.strings -o output.xml
  langcodec convert -i input.csv -o output.strings
  langcodec convert -i input.json -o output.xcstrings
  ```

  The convert command automatically detects input and output formats from file extensions.
  For JSON files, it will try multiple parsing strategies:
  - Standard Resource format (if supported by langcodec)
  - JSON key-value pairs (for custom JSON formats)

- **Merge** multiple files of the same format:

  ```sh
  langcodec merge -i file1.csv file2.csv -o merged.csv --lang en --strategy last
  langcodec merge -i en.lproj/Localizable.strings fr.lproj/Localizable.strings -o merged.strings --lang en
  ```

  - `--strategy` can be `last` (default), `first`, or `error` (fail on conflict).
  - `--lang` is required for formats that need a language code (e.g., CSV, .strings).

- **Debug**: Output a file's parsed representation as JSON:

  ```sh
  langcodec debug -i input.csv --lang en
  langcodec debug -i input.strings --lang en -o output.json
  ```

- **View**: Pretty-print entries in a localization file:

  ```sh
  langcodec view -i input.strings --lang en
  ```

#### Notes

- For CSV files, the language code (`--lang`) is required for most operations.
- All commands support Apple `.strings`, `.xcstrings`, Android `strings.xml`, and CSV.
- The convert command also supports JSON files with key-value pairs.
- The CLI will error if you try to merge files of different formats.

#### Custom Formats

The CLI supports additional custom formats for specialized use cases:

**JSON Language Map** (`json-language-map`):

```json
{
    "key": "hello_world",
    "en": "Hello, World!",
    "fr": "Bonjour, le monde!"
}
```

**JSON Array Language Map** (`json-array-language-map`):

<!-- cspell:disable -->
```json
[
    {
        "key": "hello_world",
        "en": "Hello, World!",
        "fr": "Bonjour, le monde!"
    },
    {
        "key": "welcome_message",
        "en": "Welcome to our app!",
        "fr": "Bienvenue dans notre application!"
    }
]
```
<!-- cspell:enable -->

**YAML Language Map** (`yaml-language-map`):

```yaml
key: hello_world
en: Hello, World!
fr: Bonjour, le monde!
```

Use these formats with the `--input-format` flag:

```sh
langcodec convert -i input.json -o output.xcstrings --input-format json-language-map
langcodec convert -i input.json -o output.xcstrings --input-format json-array-language-map
langcodec convert -i input.yaml -o output.xcstrings --input-format yaml-language-map
```

---

## Data Model

At the core of `langcodec` is the `Resource` struct—an expressive, format-agnostic model for localization data.
See [`src/types.rs`](src/types.rs) for details.

```rust
pub struct Resource {
    pub metadata: Metadata,
    pub entries: Vec<Entry>,
}
```

Each `Entry` supports singular and plural translations, comments, status, and custom fields.

---

## Error Handling

All public APIs use the crate's own `Error` enum, which provides meaningful variants for parsing, I/O, and format mismatches.

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
See [CONTRIBUTING.md](CONTRIBUTING.md) (to be written) for guidelines.

---

## License

This project is licensed under the MIT License.

---

## Acknowledgements

- Inspired by the need for universal localization tooling in cross-platform apps
- Built with love in Rust

---

## Status and Roadmap

`langcodec` aims to be a universal, format-agnostic localization toolkit that simplifies working with diverse localization file formats. The current focus is on stabilizing core features, expanding format support, and developing a user-friendly CLI. We welcome your issues, feature requests, and discussions at the project's issue tracker.
