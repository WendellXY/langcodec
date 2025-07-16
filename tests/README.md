# Test Data

This directory contains test files for the langcodec project.

## Structure

```text
tests/
└── data/
    ├── lib/                   # Library crate test files
    │   ├── sample_en.strings      # English Apple .strings file
    │   ├── sample_fr.strings      # French Apple .strings file
    │   ├── sample_android.xml     # Android strings.xml file
    │   ├── sample_conflict.strings # Apple .strings with duplicate keys
    │   └── sample.csv             # CSV format file
    └── cli/                   # CLI crate test files
        ├── cli_sample1.strings
        ├── cli_sample2.strings
        ├── cli_sample_android.xml
        ├── cli_sample_conflict.strings
        └── cli_sample.csv
```

## Usage

### Library Tests

Use files in `tests/data/lib/` for library crate tests.

### CLI Tests

Use files in `tests/data/cli/` for CLI-specific tests.

## Adding New Test Files

1. **Library tests**: Add to `tests/data/lib/`
2. **CLI tests**: Add to `tests/data/cli/`
3. **Format-specific tests**: Create subdirectories as needed (e.g., `tests/data/lib/android/`)

## Naming Convention

- `sample_*.{ext}` - General test files
- `cli_sample*.{ext}` - CLI-specific test files
- `*_conflict.*` - Files with duplicate/conflicting keys
- `*_expected.*` - Expected output files for tests
