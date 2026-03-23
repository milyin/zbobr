# Checklist item: add-label-constants-to-state

## Changes

**File:** `zbobr-api/src/task.rs`

Added to `impl State`:
- `pub const LABEL_DONE: &'static str = "done"` (and PAUSE, READY, PENDING, RUNNING)
- `pub const ALL_LABEL_NAMES: &'static [&'static str]` — array of all 5 label name constants
- `pub fn label_name(&self) -> Option<&'static str>` — returns the label name for a state variant, or `None` for `Empty`/`Unknown`

These follow the same pattern as `Pipeline::MAIN`/`MERGE` + `as_str()`.

## Tests added

3 new tests in `task::tests`:
- `state_label_name_returns_correct_values` — verifies all state variants return expected label names
- `state_label_name_matches_label_constants` — verifies label_name() returns the same constants
- `state_all_label_names_covers_all_variants` — verifies ALL_LABEL_NAMES has all 5 entries

## Verification

- All 8 tests in zbobr-api pass
- Full `cargo check` succeeds with no warnings
- Committed as d2e0702
