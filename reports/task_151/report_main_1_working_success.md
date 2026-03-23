# Task 151: Setup Signal Labels - Implementation Report

## Changes Made

### 1. `zbobr-api/src/backend.rs`
- Changed `TaskBackend::setup` signature from `setup(&self, force: bool)` to `setup(&self, force: bool, signal_labels: &[String])` to accept the list of required signal labels.

### 2. `zbobr-task-backend-github/src/github.rs`
- Added `delete_label(&self, name: &str)` method using `DELETE /repos/{owner}/{repo}/labels/{name}` with `retry_github`, following the same pattern as `update_label`.
- Implemented signal label sync in `setup()`:
  - Filters existing labels for `signal:*` prefix
  - Deletes obsolete signal labels (exist in repo but not in required set)
  - Creates missing signal labels (required but not in repo)
  - Updates existing signal labels when `force` is true
  - Uses `SIGNAL_LABEL_COLOR = "c2e0c6"` constant
- Updated outer `ZbobrTaskBackendGithub` impl to pass through `signal_labels`.

### 3. `zbobr-dispatcher/src/lib.rs`
- In `setup_repository()`, computes required signal labels from workflow config:
  - `signal:go_{stage}` for all stages across all pipelines
  - `signal:call_{pipeline}` for all pipelines
  - `signal:return` and `signal:return_failure`
  - Sorted and deduplicated before passing to backend

### 4. Updated all other `TaskBackend::setup` implementations
- `zbobr-dispatcher/src/backend.rs` (DummyBackend)
- `zbobr-dispatcher/src/task.rs` (test backend)
- `zbobr-task-backend-fs/src/fs.rs` (both inner and outer FS backends)

## Verification
- `cargo build` succeeds
- `cargo test` passes all 96 tests with no regressions
- Committed as c7be1dd on branch `zbobr_fix-151-setup-labels`