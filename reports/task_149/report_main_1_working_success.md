# Task 149: Add stage counting — Implementation Report

## Summary
Added `stage_count: u64` field to Task that is automatically incremented each time a stage is entered. Implementation follows the `pipeline_run_id` analog pattern exactly.

## Changes (commit c8d407f, already on branch)

### 1. API struct (`zbobr-api/src/task.rs`)
- Added `stage_count: u64` field with `#[serde(default)]` to `Task` struct

### 2. Dispatcher logic (`zbobr-dispatcher/src/`)
- **task.rs**: Added `increment_stage_count()` method on `TaskSession`; protected `stage_count` in `RoleSession::modify_task_mcp` so agents cannot tamper with it
- **cli.rs**: Call `increment_stage_count()` before each stage runs in `CliStageRunner`, and in `handle_call_stage` for pipeline calls
- **prompts.rs**, **workflow.rs**: Updated test Task construction sites with `stage_count: 0`

### 3. FS backend (`zbobr-task-backend-fs/src/fs.rs`)
- Added `stage_count` to `TaskFile` struct with serde default
- Mapped field in both `to_task()` and `from_task()` conversions
- Updated test construction site

### 4. GitHub backend (`zbobr-task-backend-github/src/github.rs`)
- Parse `stage_count` from params map in `parse_task()`
- Serialize `stage_count` to params map in `task_to_params()` (only when > 0)

### 5. CLI (`zbobr/src/commands.rs`)
- Updated dummy task construction with `stage_count: 0`

## Verification
- `cargo check` — compiles cleanly
- `cargo test` — all 96 tests pass (7 GitHub integration tests ignored as expected)
- No uncommitted changes