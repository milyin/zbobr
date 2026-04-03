# Implementation Complete

## Changes Made

### zbobr-dispatcher/src/cli.rs
- Added `TaskListEntry` struct (id, stage_count, state, title) with `serde::Serialize` + `Debug` derives and `From<&Task>` impl
- Added `select_ready_task(tasks: &[Task]) -> Option<&Task>` — filters done/paused tasks, returns the one with highest `stage_count`

### zbobr-dispatcher/src/lib.rs
- Exported `TaskListEntry` and `select_ready_task` from the crate

### zbobr/src/commands.rs
- Updated imports to include `TaskListEntry` and `select_ready_task`
- Added `--json` (bool) and `--select` (bool) flags to `TaskSubcommand::List`
- Added `--json` (bool) flag to `TaskSubcommand::Show`
- Updated `List` match arm:
  - Default: compact one-line format `{id}\t{stage_count}\t{state:?}\t{title}`
  - `--json`: serializes `Vec<TaskListEntry>` as pretty JSON
  - `--select`: calls `select_ready_task`, prints id or exits with code 1
- Updated `Show` match arm:
  - With id + `--json`: serializes full `Task` as pretty JSON
  - No id + `--json`: serializes `Vec<TaskListEntry>` as pretty JSON (consistent with list)
- Updated `run_without_backends` for `Show { id: None, json }` to handle JSON flag on sample task

## Verification
- `cargo build` — succeeded (all 3 modified crates)
- `cargo test -p zbobr-dispatcher -p zbobr` — all tests pass
- Pre-existing failures in `zbobr-task-backend-github` (rustls CryptoProvider) are unrelated to these changes