# Planning complete: move label controls to parameters

## Approach

Move `pipeline`, `stage`, and `signal` from GitHub issue labels into the PARAMETERS section of the issue body. Keep `state:*` and `flag:*` labels unchanged.

## Chosen analog

`pipeline_run_id` already follows this exact pattern — written via `task_to_string_params()` and read via `params_map` in `issue_to_task()`. The three new fields follow the same approach.

## Key design decisions

1. **`labels_to_state()` signature change**: Now accepts `pipeline: Option<&str>` and `stage: Option<&str>` from the params section, rather than parsing them from labels.

2. **`apply_signal_change()` deleted entirely**: Signal is now persisted through the normal task body update path (`task_to_string_params` → `serialize_description_full`), not via a separate label API call.

3. **`setup()` trait simplified**: `signal_labels` parameter removed from `TaskBackend::setup()` since no signal labels need to be pre-created in the repository.

4. **No migration**: Existing `pipeline:*`, `stage:*`, `signal:*` labels on old issues will simply be left as-is (harmless noise). No cleanup logic added.

## Files modified
- `zbobr-task-backend-github/src/github.rs` (main changes)
- `zbobr-api/src/backend.rs` (trait signature)
- `zbobr-task-backend-fs/src/fs.rs` (trait impl)
- `zbobr-dispatcher/src/lib.rs` (caller)
- `zbobr-dispatcher/src/backend.rs` and `src/task.rs` (test mocks)
