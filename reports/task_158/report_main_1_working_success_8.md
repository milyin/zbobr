# Work Report: Move label code to backend, fix callers and tests

## Commits

1. **7f1ef73** - `refactor: move label constants from State API to GitHub backend module`
   - Moved `LABEL_DONE/PAUSE/READY/PENDING/RUNNING`, `ALL_LABEL_NAMES`, and `label_name()` from `State` impl in `zbobr-api/src/task.rs` to module-level constants (`STATE_LABEL_DONE`, etc.) and `ALL_STATE_LABEL_NAMES` in `zbobr-task-backend-github/src/github.rs`
   - Removed unused `state_label_name()` function (was only used in now-removed tests)
   - Removed 3 label-related tests from API crate

2. **802e98c** - `refactor: fix compile errors from removed State Display/PartialEq<str>`
   - Changed all `{}` / `to_string()` usages of `State` in logging to `{:?}`
   - Changed `task.state == "DONE"` to `task.state.is_done()` in `cleanup.rs` and `fs.rs`
   - Changed `TaskFile.state` from `String` to `State` in fs backend for direct serde support
   - Added `Default` derive for `State` (defaults to `Empty`)

3. **665e5bb** - `refactor: update test assertions to use State enum instead of string comparisons`
   - Replaced all `assert_eq!(task.state, "DONE")` with `State::Done`, etc.
   - Replaced `task.state.ends_with("_PENDING")` with `task.state.is_pending()`
   - Added `State` import to test helpers

## Files changed
- `zbobr-api/src/task.rs` — removed label constants/methods/tests, added Default derive
- `zbobr-task-backend-github/src/github.rs` — added label constants, updated references
- `zbobr-task-backend-fs/src/fs.rs` — changed state field to State type, use is_done()
- `zbobr-dispatcher/src/cli.rs` — {:?} for State in logging
- `zbobr-dispatcher/src/cleanup.rs` — is_done() instead of == "DONE"
- `zbobr-dispatcher/src/workflow.rs` — {:?} for State in logging
- `zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs` — State enum in assertions
- `zbobr-dispatcher/tests/mcp_integration/env.rs` — State methods in conditions

## Test results
All 103 tests pass, zero failures, clean compilation with no warnings.
