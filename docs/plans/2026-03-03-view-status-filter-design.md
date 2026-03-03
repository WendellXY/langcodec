# View Status Filter Design

Date: 2026-03-03
Owner: CLI team

## Summary

Add status-based filtering to the existing `langcodec view` command so users can list untranslated or review-needed entries directly from large localization files.

This v1 extends `view` (no new `filter` subcommand) with:

- `--status <csv>`
- `--keys-only`
- `--json`

The design keeps existing `view` behavior unchanged when no new flags are used.

## Goals

- Let users answer:
  - Which keys are still untranslated?
  - Which keys are `new` / `needs_review` for language X?
- Support scripting and CI workflows via stable machine-readable output.
- Preserve current UX for users who do not opt into filtering.

## Non-Goals (v1)

- No separate `filter` subcommand.
- No heuristic status inference beyond current parser outputs.
- No general query language (for example `--where`).

## CLI Contract

Extend `view` options:

- `--status <csv>`
  - Comma-separated statuses from: `translated|needs_review|new|do_not_translate|stale`
  - Accepts normalized forms where `-` and spaces map to `_`.
- `--keys-only`
  - Text mode: prints only key lines for matches.
- `--json`
  - Emits machine-readable filtered output.

Works with existing `view` flags:

- `--lang` narrows results to one language first.
- `--full` controls text truncation only.
- `--check-plurals` remains supported and keeps existing exit-code behavior.

## Behavior Semantics

### Filtering scope

Filtering is per language entry, not global key-level filtering.

- With `--lang`, each key appears at most once for that language.
- Without `--lang`, the same key may appear multiple times across languages.

### Strict mode

When `--status` is provided:

- Non-strict mode: apply filtering to current parsed statuses.
- Strict mode (`--strict`): fail for input formats without explicit status metadata.
  - v1 explicit-status format: `.xcstrings`
  - Other formats fail strict status-filter requests.

Strict mode validation is specific to status filtering. Existing strict parsing behavior remains unchanged.

### Exit codes

- `0`: success (including zero matches)
- `1`: invalid status value, strict unsupported status-filter format, or other command error
- `2`: plural validation failure with `view --check-plurals` (existing behavior)

## Output Design

### Text mode (`view` without `--json`)

- Default: existing detailed output, but only for matched entries if `--status` is set.
- `--keys-only`:
  - with `--lang`: one key per line (`key`)
  - without `--lang`: one line per match with language disambiguation (`lang<TAB>key`)

### JSON mode (`--json`)

`--json` without `--keys-only`:

```json
{
  "summary": {
    "total_matches": 3,
    "languages": ["en", "fr"],
    "statuses": ["new", "needs_review"]
  },
  "entries": [
    {
      "lang": "fr",
      "key": "welcome_title",
      "status": "new",
      "type": "singular",
      "value": "",
      "comment": null
    }
  ]
}
```

`--json --keys-only`:

```json
{
  "summary": {
    "total_matches": 3,
    "languages": ["en", "fr"],
    "statuses": ["new", "needs_review"]
  },
  "keys": [
    { "lang": "fr", "key": "welcome_title" },
    { "lang": "en", "key": "welcome_title" }
  ]
}
```

If `--lang` is set, `keys` may be emitted as plain key strings to reduce verbosity.

## Architecture and Components

### `langcodec-cli/src/main.rs`

- Extend `Commands::View` with:
  - `status: Option<String>`
  - `keys_only: bool`
  - `json: bool`
- Build and pass a `ViewOptions` struct to view rendering.
- Enforce strict status-filter compatibility check (`--status` + `--strict`).

### `langcodec-cli/src/view.rs`

- Add `ViewOptions` and parsed status set type.
- Add helpers:
  - status list parsing/validation
  - entry matching by status set
  - filtered result collection
  - text renderers (detailed, keys-only)
  - JSON renderers (entries, keys-only)
- Keep existing rendering path for backward compatibility when no filtering/output flags are used.

## Data Flow

1. Parse CLI args (`view` + new flags).
2. Read codec using existing read path (`load_codec_for_readonly_command`).
3. If `--status`, parse and validate status list.
4. If strict + status filter + unsupported format, return error.
5. Filter resources/entries by optional `--lang` and status list.
6. Render:
   - text detailed
   - text keys-only
   - JSON entries
   - JSON keys-only
7. Run plural validation if requested.

## Error Handling

- Invalid status token: explicit error listing accepted statuses.
- Unknown language (`--lang`): existing error behavior unchanged.
- Strict unsupported format for status filtering: clear error that explicit status metadata is required.
- JSON serialization failure: return command error with context.

## Testing Strategy

Add `langcodec-cli/tests/view_status_cli_tests.rs` covering:

- filter single status
- filter multiple statuses
- language-scoped filtering (`--lang`)
- multi-language output when `--lang` omitted
- text `--keys-only` output (with and without `--lang`)
- `--json` structure and values
- `--json --keys-only` payload shape
- invalid status rejection
- strict + status on non-explicit-status format fails
- strict + status on `.xcstrings` succeeds

Regression checks:

- Existing `view` tests still pass unchanged.
- Existing plural validation exit behavior remains intact.

## Documentation Updates

- Update `langcodec-cli/README.md` `view` section with new flags and examples.
- Update root `README.md` quick examples.
- Optional changelog entry under unreleased section.

## Rollout

Ship as a backward-compatible CLI enhancement with no migration needed.

## Open Follow-ups (post-v1)

- Add generalized query/filter syntax (for example future `--where`).
- Consider extracting a shared query engine if additional filter dimensions are added.
- Revisit key payload normalization for JSON keys-only mode for cross-tool consistency.
