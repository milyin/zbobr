# Plan: Add `--select` flag for `task process`

## Context

The `task list --select` flag already selects the highest-priority runnable task and prints its ID. The user wants the same selection logic available in `task process`, so that `task process --select` automatically picks the highest-priority task and processes it in one step (instead of requiring two commands: `task list --select` then `task process <id>`).

## Approach

**Analog:** `task list --select` in `zbobr/src/commands.rs` (lines 309-336) — same selection pattern, same `select_runnable_task` utility.

### Changes in `zbobr/src/commands.rs`

1. **Add `--select` flag to the `Process` variant** (line ~130): Add `#[arg(long)] select: bool` alongside the existing `task: Option<u64>`.

2. **Update the handler** (line ~430): When `select` is true, list all tasks, call `select_runnable_task` (already imported), and process the selected task. When no runnable task is found, exit with code 1 (matching `list --select` behavior). When `select` is false, fall back to the existing `require_task_id` path. If both `task` and `--select` are provided, return an error.

## Verification

- `cargo build` to confirm compilation
- `cargo test` to confirm existing tests pass
- Manual: `task process --select` with runnable tasks should process the highest-priority one
- Manual: `task process --select` with no runnable tasks should exit with code 1
- Manual: `task process --select 123` should error (conflicting arguments)