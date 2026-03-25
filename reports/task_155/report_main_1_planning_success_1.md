# Plan: Fix auto-pause bugs and code duplication

## Context
The initial implementation of task priority sorting and auto-pause (by `max_stage_count`) has three bugs identified during review:
1. **Stack duplication in `handle_call_stage`**: `push_stack` happens before the auto-pause check — if auto-pause triggers, the early `return Ok(())` leaves a dangling stack entry. When `apply_pause_to_state` runs later, it pushes *another* stack entry, causing duplication.
2. **Premature counter increment in `CliStageRunner::run_stage`**: `increment_stage_count` is called before the auto-pause check. The stage hasn't actually run yet, so the count is wrong if auto-pause fires.
3. **Code duplication**: The pattern `modify_task(|mut t| { t.pause = true; t })` is copy-pasted in multiple places. `TaskSession` already has `set_pause(bool)` (line 327 of `task.rs`) which should be used instead.

## Fixes

### Fix 1: Reorder `handle_call_stage` (cli.rs ~lines 609-632)
Move the auto-pause check **before** `push_stack`. New order:
1. `allocate_pipeline_run_id`
2. `increment_stage_count`
3. **Auto-pause check** — if triggered, `set_pause(true)` and `return Ok(())`
4. `push_stack` (only if proceeding with the call)

### Fix 2: Reorder `CliStageRunner::run_stage` (cli.rs ~lines 350-373)
Move the auto-pause check **before** `increment_stage_count`. New order:
1. Check `stage_count >= max_stage_count` — if reached, `set_pause(true)` and `return Ok(())`
2. `increment_stage_count` (only if stage will actually run)

### Fix 3: Deduplicate pause code
- **Simple pause** (`pause = true` only): replace inline `modify_task` with `task_session.set_pause(true)` — already exists at `task.rs:327`. Applies to ~2 places in cli.rs.
- **Pause + signal**: add `set_pause_with_signal(signal: Signal)` helper on `TaskSession` in `task.rs` near existing `set_pause`. Use it everywhere the pattern `{ t.pause = true; t.signal = Some(signal); t }` appears (~6 places in cli.rs: finalize_stage_session, executor timeout, detect_and_handle_worktree, etc.).

## Analog
The existing `set_pause(bool)` method at `task.rs:327` is the analog pattern for the new `set_pause_with_signal` helper.

## Key files
- `zbobr-dispatcher/src/cli.rs` — auto-pause logic reordering, replace inline pause code
- `zbobr-dispatcher/src/task.rs` — add `set_pause_with_signal` helper near line 327

## Verification
- `cargo build` — compiles
- `cargo test` — existing tests pass
- Confirm `push_stack` in `handle_call_stage` only happens after auto-pause check
- Confirm `increment_stage_count` in `run_stage` only happens after auto-pause check
- Grep `t.pause = true` to confirm no remaining inline duplicates