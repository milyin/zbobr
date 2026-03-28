# Summary

Fixed the two issues raised by the reviewer in ctx_rec_15:

## 1. Centralized ERROR_PREFIX constant

Added `pub const ERROR_PREFIX: char = '\u{274C}';` to `zbobr-api/src/backend.rs` and exported it from `zbobr-api/src/lib.rs`. Both `TaskMut::set_error` (backend.rs) and `RoleSession::set_error` (dispatcher/task.rs) now reference `ERROR_PREFIX` instead of the raw `\u{274C}` literal. The test also uses `zbobr_api::ERROR_PREFIX`.

## 2. Error-clear invariant in generic TaskMut::set_state

The `TaskMut::set_state` default impl in `zbobr-api/src/backend.rs` now clears `task.error` when transitioning from a non-running state into a running state (`!task.state.is_running() && state.is_running()`). This ensures all backend implementations get the invariant for free. The dispatcher's `RoleSession::set_state` already has its own override with additional `confirm` logic and retains the same error-clear condition.

## Verification

- `cargo build` succeeds with no warnings
- All tests pass