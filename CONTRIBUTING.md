# Contributing to langcodec

Thank you for your interest in contributing to langcodec! This document provides guidelines for contributing to the project.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git

### Development Setup

**1. Fork and clone the repository:**

```bash
git clone https://github.com/WendellXY/langcodec.git
cd langcodec
```

**2. Install dependencies and run tests:**

```bash
cargo test
cargo clippy
```

## Development Guidelines

### Code Style

- Follow Rust's official style guide
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for common issues
- Write comprehensive tests for new features

### Adding New Formats

To add support for a new localization format:

1. Create a new module in `langcodec/src/formats/`
2. Implement the `Parser` trait for your format
3. Add `From`/`TryFrom` conversions to/from `Resource`
4. Update `FormatType` enum in `formats/mod.rs`
5. Add tests in the appropriate test module

### Testing

- Write unit tests for all new functionality
- Add integration tests for format conversions
- Include test data files in `tests/data/`
- Ensure all tests pass before submitting PR

### Documentation

- Document all public APIs
- Update README.md for new features
- Add examples in doc comments
- Update the format support table in README.md

## Submitting Changes

**1. Create a feature branch:**

 ```bash
 git checkout -b feature/your-feature-name
 ```

**2. Make your changes and commit:**

 ```bash
 git add .
 git commit -m "Add: brief description of changes"
 ```

**3. Push and create a pull request:**

 ```bash
 git push origin feature/your-feature-name
 ```

## Issue Reporting

When reporting issues:

1. Use the issue template
2. Include steps to reproduce
3. Provide sample input files if relevant
4. Include error messages and stack traces
5. Specify your Rust version and platform

## Code Review Process

1. All changes require review
2. Address review comments promptly
3. Ensure CI checks pass
4. Update documentation as needed

## Release Process

1. Update version numbers in `Cargo.toml` files
2. Update CHANGELOG.md
3. Create a release tag
4. Publish to crates.io

Thank you for contributing to langcodec!
