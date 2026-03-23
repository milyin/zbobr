# Implementation Report

## Changes Made (single commit: 7bca76e)

### 1. `ensure_label_exists()` helper (github.rs ~line 474)
Added a new method that calls `octocrab.issues().create_label()` directly and matches on the result:
- `Ok(_)` → label created, return Ok
- `Err(GitHub { status_code: 422 })` → label already exists, silently return Ok
- Other errors → propagate as anyhow::Error

This avoids the retry logic in `create_label()` which would convert the 422 into a hard error.

### 2. Dynamic label creation in `apply_state_change()` (github.rs ~line 365)
Before calling `add_labels`, iterates over `new_labels` and calls `ensure_label_exists()` for each with the correct color from `state_label_color()` and a description. This ensures dynamically-generated labels like `pipeline:custom_name` and `stage:custom_name` exist in the repo before being assigned to issues.

### 3. Fixed `state:pending` color (github.rs line 287)
Changed from `d4c5f9` (lavender/purple) to `d3d3d3` (gray) per task requirements.

### 4. Verification
- `cargo test -p zbobr-task-backend-github`: 11 tests passed
- `cargo clippy -p zbobr-task-backend-github`: only pre-existing warnings (unrelated to changes)
