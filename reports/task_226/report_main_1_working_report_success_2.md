# Fix: pause-without-status constraint in TaskSession::set_state

## Problem
The reviewer identified that `TaskSession::set_state()` in `zbobr-dispatcher/src/task.rs` could set `task.pause = true` (when `task.confirm == true` and state changes) without setting any status message, violating the requirement that "pause cannot be set without explanation."

## Fix Applied
Three files changed:

### zbobr-api/src/backend.rs
Added `PAUSE_PREFIX` constant (`⏸`, U+23F8) for pipeline/confirmation-triggered pause status messages, alongside the existing `ERROR_PREFIX` and `QUESTION_PREFIX`.

### zbobr-api/src/lib.rs
Exported the new `PAUSE_PREFIX` constant from the API crate.

### zbobr-dispatcher/src/task.rs
Fixed `TaskSession::set_state()` to compute a formatted status string (using `format_status(PAUSE_PREFIX, ...)`) before the closure, then set it atomically alongside `pause = true` when the confirmation-triggered pause fires.

## Verification
- `cargo fmt -- --check`: clean
- `cargo test`: 105/105 tests pass
- `cargo build`: succeeds
