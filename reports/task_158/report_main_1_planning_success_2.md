# Planning Report: Replace Hardcoded Label Literals with Type-Derived Values

## Approach

Replace all hardcoded label string literals (`"state:done"`, `"pipeline:main"`, etc.) in `github.rs` with values derived from type constants and methods. The analog is the existing `Pipeline` type which already exposes `MAIN`/`MERGE` constants and `as_str()`.

## Key Design Decisions

1. **Label name constants on `State`** (`LABEL_DONE = "done"`, etc.) in `task.rs` — co-locates the canonical label names with the enum definition, single source of truth. These are lowercase (distinct from the existing uppercase `DONE`/`PAUSE` constants used for milestone serialization).

2. **Prefix constants** (`STATE_PREFIX = "state:"`, etc.) on the GitHub impl — since label prefixes are GitHub-backend-specific.

3. **`ALL_LABEL_NAMES` array** on `State` — enables programmatic enumeration in `setup()` instead of manual listing.

4. **`label_name()` method** — allows `state_to_labels()` to be rewritten without match arms on hardcoded strings.

## Files Modified

- `zbobr-api/src/task.rs` — Add constants, array, and method to `impl State`
- `zbobr-task-backend-github/src/github.rs` — Add prefix constants, rewrite 5 functions
- `zbobr/src/init.rs` — Update worker and reviewer default prompts

## Risk Assessment

Pure refactoring — label format on GitHub is unchanged. All existing tests should pass without modification.
