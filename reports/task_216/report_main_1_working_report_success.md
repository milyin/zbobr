# Task 216: Move flag labels to parameters — Completed

## Changes made in `zbobr-task-backend-github/src/github.rs`

### 1. `issue_to_task` — read flags from params
Replaced label-iteration for `pause`/`confirm` with params_map lookup:
```rust
let pause = params_map.get(FLAG_PAUSE).map(|s| s == "true").unwrap_or(false);
let confirm = params_map.get(FLAG_CONFIRM).map(|s| s == "true").unwrap_or(false);
```

### 2. `task_to_string_params` — write flags as params
Added serialization of `pause` and `confirm` into the PARAMETERS section (only when `true`), using `FLAG_PAUSE`/`FLAG_CONFIRM` constants as keys.

### 3. `apply_flag_change` removed
Deleted the method and its call site in `modify_task_internal`. Also removed tracking of `original_pause`/`original_confirm`.

### 4. `setup()` — flag label management removed
Removed the "Create flag labels" loop. Updated the obsolete-label cleanup to only delete `state:*` labels (no longer tracks `flag:*`).

### 5. Constants and helpers removed
- `FLAG_PREFIX`
- `ALL_FLAG_NAMES`
- `flag_to_label()` method
- `label_to_flag()` method

`FLAG_PAUSE` and `FLAG_CONFIRM` are retained as param-key constants (comment updated to "Flag parameter name constants").

### 6. Tests updated
Replaced the old commented-out test block with three new tests in `flag_tests` module:
- `issue_to_task_reads_pause_from_params`
- `issue_to_task_reads_confirm_from_params`
- `task_to_string_params_includes_flags_when_set`

All 18 tests in the crate pass.
