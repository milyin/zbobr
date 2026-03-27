## Summary

Moved `pipeline`, `stage`, and `signal` from GitHub labels to issue body parameters. All changes in a single commit on branch `zbobr_fix-201-move-label-controls-to-params`.

## Changes

### `zbobr-task-backend-github/src/github.rs`
- Removed `PIPELINE_PREFIX`, `STAGE_PREFIX`, `SIGNAL_PREFIX` constants
- Removed `signal_to_label()` and `label_to_signal()` helper functions
- `state_to_labels()`: now emits only `state:*` label (no `pipeline:`/`stage:`)
- `labels_to_state()`: accepts `pipeline_param: Option<&str>` and `stage_param: Option<&str>` instead of reading from labels
- `state_label_color()`: removed dead `PIPELINE_PREFIX`/`STAGE_PREFIX` branch
- `apply_state_change()`: only removes/adds `state:*` labels
- Removed `apply_signal_change()` entirely
- `modify_task_internal()`: removed `original_signal` tracking and `apply_signal_change` call
- `issue_to_task()`: reads `pipeline`, `stage`, `signal` from params_map; passes them to `labels_to_state()`
- `task_to_string_params()`: serializes `pipeline`/`stage` from State and `signal` as string params
- `setup()`: removed signal label sync code and pipeline label creation; removed `signal_labels` parameter

### `zbobr-api/src/backend.rs`
- `TaskBackend::setup()` trait: removed `signal_labels: &[String]` parameter

### `zbobr-task-backend-fs/src/fs.rs`
- Updated both `setup()` implementations to match new signature

### `zbobr-dispatcher/src/backend.rs`, `src/task.rs`
- Updated stub `setup()` implementations

### `zbobr-dispatcher/src/lib.rs`
- Removed `SIGNAL_PREFIX` constant and signal_labels computation
- Updated `setup()` call to pass only `force`

## Build & Tests
- `cargo build`: success (1 pre-existing warning about `delete_label` unused)
- `cargo test`: all 98+ tests pass, 0 failures
