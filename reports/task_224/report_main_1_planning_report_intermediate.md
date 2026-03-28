# Plan: Move Parameter Constants to GitHub Backend

## Problem
All 14 `PARAM_*` string constants (e.g. `PARAM_DESTINATION_BRANCH`, `PARAM_PR_URL`, etc.) are defined in `zbobr-api/src/task.rs` and re-exported from `zbobr-api/src/lib.rs`. These constants are internal storage keys for the GitHub Issues serialization format — they are not part of the public API. The FS backend doesn't need them at all.

## Affected Files
- `zbobr-api/src/task.rs` — defines the constants
- `zbobr-api/src/lib.rs` — re-exports them
- `zbobr-task-backend-github/src/github.rs` — imports and uses all 14 constants
- `zbobr-task-backend-fs/src/fs.rs` — imports and uses only `PARAM_PR_URL` (all other fields already first-class in `TaskFile`)
- `zbobr-dispatcher/src/prompts.rs` — imports `PARAM_DESTINATION_BRANCH`, `PARAM_DESTINATION_REPOSITORY`, `PARAM_WORK_BRANCH` to define `VAR_*` template constants

## Proposed Changes

### 1. GitHub backend — add local params module
Create `zbobr-task-backend-github/src/params.rs` with all 14 `PARAM_*` constants (crate-private). Update `github.rs` to import from the local module instead of zbobr-api.

### 2. zbobr-api — remove PARAM_* constants
Delete all 14 constants from `task.rs` and remove their re-exports from `lib.rs`.

### 3. FS backend — replace parameters HashMap with specific field
`TaskFile` currently has a `parameters: HashMap<String, String>` that holds only `pr_url`. Replace it with `pr_url: Option<String>` as a first-class field (consistent with how `destination_repository`, `destination_branch`, `work_branch` are already promoted). Update `to_task()` and `from_task()` accordingly. Remove the `PARAM_PR_URL` import.

### 4. Dispatcher — use literal strings for VAR_* constants
`VAR_DESTINATION_REPOSITORY`, `VAR_DESTINATION_BRANCH`, `VAR_WORK_BRANCH` are currently defined as `= PARAM_*` from zbobr-api. These are prompt template variable names — the link to PARAM_* was incidental (same string value). Replace with literal strings and remove PARAM_* imports.

## Rationale
- PARAM_* constants are GitHub-specific serialization keys; keeping them in the public API creates false coupling
- The FS backend used a HashMap only for `pr_url` — this is already inconsistent with the other routing fields being first-class; fixing it improves consistency
- The dispatcher VAR_* constants represent task fields for template substitution — they should stand alone, not depend on a backend's storage format
