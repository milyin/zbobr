# Implementation Report: Task Priority Fix & Counter Limit

## Changes Made (11 files, 1 commit: e5fc702)

### 1. Config: `max_task_stage_count` field
- **zbobr-api/src/config.rs**: Added `max_task_stage_count: u64` with `#[arg(default_value = "20")]` to `ZbobrDispatcherConfig`, plus default value `20` in `Default` impl.
- **zbobr/src/init.rs**: Added `max_task_stage_count: None` to default config TOML template.

### 2. Task: `max_stage_count` field
- **zbobr-api/src/task.rs**: Added `pub max_stage_count: u64` with `#[serde(default)]` to `Task` struct.
- Updated all Task construction sites (6 files) to include `max_stage_count: 0`.

### 3. Field Protection
- **zbobr-dispatcher/src/task.rs**: `RoleSession::modify_task` now saves/restores `max_stage_count` alongside `stage_count`, preventing agent modification.

### 4. Set on Task Creation
- **zbobr-dispatcher/src/lib.rs**: `create_task_with_confirm` sets `task.max_stage_count = self.config.max_task_stage_count`.

### 5. Auto-Pause on Limit
- **zbobr-dispatcher/src/cli.rs**: After each `increment_stage_count()` call (in `CliStageRunner::run()` and `handle_call_stage`), checks `stage_count >= max_stage_count` (when `max_stage_count > 0`). If exceeded, sets `task.pause = true`, logs warning, and returns early.

### 6. Task Priority Sorting
- **zbobr-dispatcher/src/cli.rs**: After collecting `all_tasks` in the manager loop, sorts by `stage_count` descending so tasks closest to completion are processed first.

### 7. Backend Persistence
- **zbobr-task-backend-fs/src/fs.rs**: Added `max_stage_count` to `TaskFile` struct and both `to_task`/`from_task` conversions.
- **zbobr-task-backend-github/src/github.rs**: Added `max_stage_count` to params parsing and serialization.

## Verification
- `cargo build` — succeeds
- `cargo test` — all tests pass (17 API tests + dispatcher tests)
