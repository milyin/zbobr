# Plan: Setup Signal Labels in GitHub Backend

## Approach
Change `TaskBackend::setup` trait signature to accept `signal_labels: &[String]`. The dispatcher computes required signal labels from `WorkflowConfig` (go/call/return/return_failure signals) and passes them to the backend. The GitHub backend syncs labels: deletes obsolete `signal:*` labels, creates missing ones, and updates existing ones on force.

## Analog
Flag label setup pattern in `github.rs:511-530` — same create/update/skip logic, extended with delete for obsolete labels.

## Key Design Decisions
- Signal labels derived from config: `signal:go_{stage}` for all stages, `signal:call_{pipeline}` for all pipelines, plus `signal:return` and `signal:return_failure`
- Set-based comparison to combine operations and avoid deleting/recreating same labels
- New `delete_label` method needed (doesn't exist yet in GitHub backend)
- FS backend and test backends just ignore the new parameter

## Files Modified
1. `zbobr-api/src/backend.rs` — trait signature change
2. `zbobr-task-backend-github/src/github.rs` — add `delete_label`, implement signal sync in `setup()`
3. `zbobr-dispatcher/src/lib.rs` — compute signal labels in `setup_repository()`
4. `zbobr-dispatcher/src/backend.rs` — DummyBackend signature update
5. `zbobr-dispatcher/src/task.rs` — test backend signature update
6. `zbobr-task-backend-fs/src/fs.rs` — FS backend signature update (2 impls)