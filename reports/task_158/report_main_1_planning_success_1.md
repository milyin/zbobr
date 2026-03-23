# Plan: Fix dynamic label creation and color mismatch

## Root Cause Analysis

The review found two issues in the milestone-to-labels migration:

1. **Critical — Dynamic label creation missing**: `apply_state_change()` calls `add_labels` for dynamically-generated labels like `pipeline:custom_name` or `stage:custom_name`, but these labels don't exist in the repo. Only hardcoded labels (`pipeline:main`, `pipeline:merge`, and the 5 `state:` labels) are created in `setup()`. The analog pattern (signal/flag labels) works because those are pre-created in `setup()`, but pipeline/stage labels are unbounded.

2. **Minor — Color mismatch**: `state:pending` uses `d4c5f9` (lavender/purple) but the task requires "pending is gray".

## Chosen Approach

Add an `ensure_label_exists` helper that tries to create a label and silently ignores "already exists" (422) errors. Call it in `apply_state_change()` before `add_labels` for each label. This is the simplest approach — no need to cache or pre-fetch labels.

## Key Design Decision

Using try-create-and-ignore-422 rather than list-then-create avoids race conditions and extra API calls. The cost is one extra API call per label per state change, but state changes are infrequent.

## File Modified

`zbobr-task-backend-github/src/github.rs` — all changes in this single file.