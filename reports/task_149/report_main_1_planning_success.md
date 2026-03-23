# Planning Report: Add `stage_count` to Task

## Approach
Add a `stage_count: u64` field to the `Task` struct that auto-increments each time a stage begins execution (both role stages and call stages). This follows the exact same pattern as the existing `pipeline_run_id` field.

## Analog chosen: `pipeline_run_id`
This is the right analog because it's also a monotonically increasing `u64` on `Task`, threaded through all backends (FS and GitHub), with `#[serde(default)]` for backward compatibility. The only difference: `stage_count` never resets (unlike `pipeline_run_id` which is restored from the stack on pipeline return).

## Key design decisions
1. **Increment on both role stages and call stages** — every stage transition counts.
2. **Monotonic lifetime counter** — never resets, unlike `pipeline_run_id`.
3. **Protected from MCP tools** — added to the save/restore list in `RoleSession::modify_task()` alongside `state` and `stack`.
4. **Backward compatible** — `#[serde(default)]` means existing tasks deserialize with 0, no migration needed.

## Files to modify
- `zbobr-api/src/task.rs` — `Task` struct
- `zbobr-task-backend-fs/src/fs.rs` — `TaskFile`, `to_task()`, `from_task()`, `create_task()`
- `zbobr-task-backend-github/src/github.rs` — `issue_to_task()`, `task_to_string_params()`
- `zbobr-dispatcher/src/task.rs` — `TaskSession::increment_stage_count()`, `RoleSession::modify_task()` protection
- `zbobr-dispatcher/src/cli.rs` — increment calls in `CliStageRunner::run()` and `handle_call_stage()`
- 4 test/utility files with Task construction sites (compiler will catch all)
