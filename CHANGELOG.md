# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [0.11.0] - 2026-03-26

### Added

- Added CLI translate coverage for Apple `.strings` and Android `strings.xml` workflows.
- Extended `annotate` to support Apple `.strings` and Android XML inputs, not just `.xcstrings`.
- Added Android strings comment round-tripping support so comment metadata survives parse/write cycles.
- Added config-driven glob expansion coverage for `translate` and `annotate` command invocations.

### Changed

- Refreshed the root README and package presentation for the current CLI and library workflows.
- Updated the GitHub release workflow triggers and build matrix configuration.

### Fixed

- Fixed config-relative glob expansion for `translate` and `annotate` so `langcodec.toml` can target multiple matching files.
- Documented the broadened annotate format support in the user-facing docs.
