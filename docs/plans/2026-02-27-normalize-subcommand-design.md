# Normalize Subcommand Design

Date: 2026-02-27
Status: Approved
Scope: `langcodec-cli` with shared normalization logic in `langcodec`

## Summary
Add a new `normalize` subcommand that canonically rewrites localization files with safe defaults. In v1, default normalization includes ordering, formatting, and placeholder normalization. It supports all standard formats: Apple `.strings`, Apple `.xcstrings`, Android `strings.xml`, CSV, and TSV.

The command writes in place by default, supports an optional `--output` path for single-file runs, and adds `--check` mode for CI drift detection.

## Goals
- Provide deterministic, repeatable formatting and ordering across supported formats.
- Offer a safe default that does not introduce semantic translation changes.
- Support CI validation via `--check` with non-zero exit on drift.
- Keep behavior consistent across formats by centralizing normalization rules.

## Non-Goals (v1)
- Supporting custom one-way transformer formats as normalize inputs.
- Enabling automatic key renaming by default.
- Broad semantic rewrites beyond safe canonicalization.

## Considered Approaches
1. Shared library normalization engine + CLI orchestration (selected)
2. CLI-only implementation
3. Writer-only implicit normalization

Selected approach rationale:
- Reusable by library and CLI consumers.
- Easier unit/integration test coverage.
- Cleaner `--check` drift reporting and future extensibility.

## CLI Surface
Proposed command:
- `langcodec normalize --inputs <...> [--lang <code>] [--output <path>] [--dry-run] [--check] [--continue-on-error]`

Rules and flags:
- Default rule set: ordering + formatting + placeholders.
- `--no-placeholders`: disable placeholder canonicalization.
- `--key-style <none|snake|kebab|camel>`: opt-in key renaming (default `none`).

Behavior:
- In-place write by default.
- `--output` allowed only when exactly one resolved input file is processed.
- `--check` performs in-memory normalization and reports drift without writing.

## Normalization Pipeline
Per input file:
1. Parse into `Codec` using existing format inference.
2. Apply rules in fixed order:
   - Ordering: stable lexical key ordering and deterministic language ordering.
   - Formatting: canonical serialization-compatible formatting and escaping without semantic rewrites.
   - Placeholders (default on): use `Codec::normalize_placeholders_in_place()`.
   - Key style (opt-in): transform keys if requested.
3. Validate key-style collisions; fail file if collisions are detected.
4. Serialize back to the target path/format.
5. Compare original vs normalized bytes:
   - `--check`: mark drift and fail process on any changed file.
   - Normal mode: write only if changed.

## Error Handling and Exit Codes
- Per-file atomicity: no write on rule failure.
- Default fail-fast; `--continue-on-error` aggregates file failures.
- Unsupported custom inputs return actionable errors.
- Exit codes:
  - `0`: success, no drift in `--check`
  - `1`: drift detected in `--check`, or runtime/validation failures
  - `2`: unchanged from existing meaning (`view --check-plurals` only)

## Reporting
- Per-file outcomes: changed, unchanged, failed.
- End summary: processed, succeeded, changed, failed.
- `--check` output includes a concise list of files with drift.

## Testing Strategy
CLI integration tests:
- In-place normalize: changed and unchanged files.
- Multi-file glob processing.
- `--output` single-file constraint.
- `--check` drift detection and exit behavior.
- `--continue-on-error` aggregated failure behavior.

Library/unit tests:
- Deterministic ordering.
- Placeholder normalization toggled on/off.
- Key-style transforms and collision failures.
- Idempotency (second normalize run yields no changes).

## Acceptance Criteria
- `normalize` works on all standard formats.
- Running normalize twice is idempotent.
- `--check` is CI-friendly and reliably detects drift.
- Key renaming is opt-in and collision-safe.
