# langcodec

**A universal localization file toolkit in Rust.**

`langcodec` provides format-agnostic parsing, conversion, and serialization for major localization formats, including Apple `.strings`, `.xcstrings`, and Android `strings.xml`. It enables seamless conversion between formats, powerful internal data modeling, and extensibility for new formats.

---

## Features

- ✨ Parse, write, and convert between multiple localization file formats
- 🦀 Idiomatic, modular, and ergonomic Rust API
- 📦 Designed for CLI tools, CI/CD pipelines, and library integration
- 🔄 Unified internal model (`Resource`) for lossless format-agnostic processing
- 📖 Well-documented, robust error handling and extensible codebase

---

## Supported Formats

| Format                | Parse | Write | Plural Support   | Comments |
|-----------------------|:-----:|:-----:|:----------------:|----------|
| Apple `.strings`      |  ✔️   |  ✔️   |   No             |  ✔️      |
| Apple `.xcstrings`    |  ✔️   |  ✔️   |   Yes            |  ✔️      |
| Android `strings.xml` |  ✔️   |  ✔️   |   No<sup>*</sup> |  ✔️      |
| CSV                   |  ✔️   |  ✔️   |   No             |  –       |

<sup>* Plural support for Android may be added in the future.</sup>

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
    codec.read_file_by_type("Localizable.strings", FormatType::Strings(None))?;

    // Manipulate resources if needed (see types.rs for Resource/Entry APIs)

    // Write as Android strings.xml
    codec.write_to_file()?;

    Ok(())
}
```

---

### CLI (Planned)

A CLI tool will be provided for easy conversion and batch processing:

```sh
langcodec convert --from Localizable.strings --to strings.xml
```

*Stay tuned for CLI usage and installation instructions!*

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

All public APIs use the crate’s own `Error` enum, which provides meaningful variants for parsing, I/O, and format mismatches.

---

## Extending

Adding a new localization format?  
Implement the `Parser` trait for your format struct in `formats/`, and add `From`/`TryFrom` conversions to and from `Resource`.  
PRs welcome!

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
