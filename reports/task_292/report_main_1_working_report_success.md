# --select flag for `task process`

## Changes Made

**File:** `zbobr/src/commands.rs`

1. Added `#[arg(long)] select: bool` field to the `Process` variant in `TaskSubcommand` enum (following the same clap pattern as `List`).

2. Updated the `Process` handler to:
   - Return an error if both `task` ID and `--select` are provided
   - When `--select` is true: list all tasks, call `select_runnable_task`, process the selected task, or exit code 1 if none found
   - When `--select` is false: fall through to existing `require_task_id` path (no behavior change)

## Verification

- `cargo build` succeeded
- `cargo test` has 3 pre-existing failures unrelated to these changes (Rustls crypto provider issue in `zbobr-task-backend-github`)