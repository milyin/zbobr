# Plan: Make task stage counter back-counted with configurable limit

## Approach
Follow the existing `pipeline_run_id` pattern as analog — a numeric field on Task, managed by TaskSession methods, displayed in parameters, with behavior driven by dispatcher config.

## Key design decisions
- `task_stage_limit` is `Option<u64>` — `None` means unlimited (backward compatible)
- Counter uses `saturating_sub(1)` to avoid underflow
- Pause is triggered only when limit is configured AND counter reaches 0
- Init happens once when `pipeline_run_id == 0` (fresh task), same block that allocates the first run ID
- Both stage entry points (regular stage in `CliStageRunner::run` and call-pipeline stage in `handle_call_stage`) get the decrement+pause logic

## Files
1. `zbobr-api/src/config.rs` — new `task_stage_limit` config field
2. `zbobr-dispatcher/src/task.rs` — `set_stage_count`, rename `increment_stage_count` → `decrement_stage_count`
3. `zbobr-dispatcher/src/cli.rs` — init on fresh task, decrement+pause at both stage entry points

Display in parameters is already handled by both backends (GitHub and FS).