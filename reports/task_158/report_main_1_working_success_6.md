# Checklist item: move-prefix-constants

## Changes made

### zbobr-api/src/task.rs
- Removed all 5 public prefix constants (STATE_PREFIX, PIPELINE_PREFIX, STAGE_PREFIX, SIGNAL_PREFIX, FLAG_PREFIX)
- Inlined string literals ("state:", "pipeline:", "stage:") in Display and From<&str> implementations

### zbobr-api/src/lib.rs
- Removed FLAG_PREFIX, PIPELINE_PREFIX, SIGNAL_PREFIX, STAGE_PREFIX, STATE_PREFIX from re-exports

### zbobr-task-backend-github/src/github.rs
- Added 5 local (private) prefix constants: STATE_PREFIX, PIPELINE_PREFIX, STAGE_PREFIX, SIGNAL_PREFIX, FLAG_PREFIX
- Updated import to no longer bring these from zbobr_api

### zbobr-dispatcher/src/lib.rs
- Added local SIGNAL_PREFIX constant
- Removed import of SIGNAL_PREFIX from zbobr_api

## Verification
- `cargo check` passes
- `cargo test` all tests pass
- Commit: 083afdc