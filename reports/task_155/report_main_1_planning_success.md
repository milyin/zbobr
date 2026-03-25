# Implementation Plan: Task Priority Fix & Stage Counter Limit

## Context
The dispatcher processes tasks in arbitrary order from `list_tasks()`. When multiple tasks are ready, there's no prioritization. Additionally, there's no mechanism to auto-pause runaway tasks. This change adds:
1. **Priority by stage_count** — tasks with higher `stage_count` (closest to completion) are processed first
2. **max_stage_count limit** — configurable limit (default 20) that auto-pauses tasks exceeding it, with per-task override support

## Analog
The `confirm` flag pattern is the closest analog: it's a per-task boolean that triggers `task.pause = true` inside `set_state()` (zbobr-dispatcher/src/task.rs:465). The pause flag is then converted to PAUSE state by `apply_pause_to_state()` in the manager loop. The new max_stage_count limit follows this same pause-flag mechanism.

## Key Design Decisions
- **Forward-counting with upper limit** (not back-counting) — simpler, matches existing `stage_count` semantics
- **Per-task `max_stage_count`** initialized from global config but individually adjustable — user can increase it for specific tasks without changing the global default
- **`max_stage_count` protected from agent modification** in RoleSession::modify_task — same pattern as stage_count, state, and stack
- **Pause via existing flag mechanism** — set `task.pause = true` and let existing `apply_pause_to_state` handle the state transition, keeping the state machine clean
- **Sort after snapshot collection** — simple `sort_by` on the already-collected `all_tasks` Vec, no changes to backend interface needed

## Files Modified
1. `zbobr-api/src/config.rs` — add `max_task_stage_count` field
2. `zbobr-api/src/task.rs` — add `max_stage_count` field to Task
3. `zbobr-dispatcher/src/task.rs` — protect field in RoleSession
4. `zbobr-dispatcher/src/lib.rs` — set field on task creation
5. `zbobr-dispatcher/src/cli.rs` — auto-pause check + sort logic