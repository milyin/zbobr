# Back-counted stage counter implementation

## Changes (4 files, commit be05c87)

### zbobr-api/src/config.rs
- Added `task_stage_limit: Option<u64>` field to `ZbobrDispatcherConfig` with doc comment
- Added `task_stage_limit: None` default in `Default` impl

### zbobr-dispatcher/src/task.rs
- Added `set_stage_count(value: u64)` method to `TaskSession` for initializing the counter from config
- Replaced `increment_stage_count` with `decrement_stage_count` — uses `saturating_sub(1)`, returns `anyhow::Result<u64>` with the new value (uses `AtomicU64` to extract value from the modify closure)

### zbobr-dispatcher/src/cli.rs
- `CliStageRunner::run()` (line ~341-348): When `pipeline_run_id == 0` (fresh task), initializes `stage_count` from `zbobr.config().task_stage_limit` using `set_stage_count`
- `CliStageRunner::run()` (line ~350): Replaced `increment_stage_count()` with `decrement_stage_count()`. If counter reaches 0 AND `task_stage_limit` is configured, sets `task.pause = true` and returns early (letting the normal pause flow handle stack push on next dispatch)
- `handle_call_stage()` (line ~595): Same decrement + pause logic applied

### zbobr/src/init.rs
- Added `task_stage_limit: None` to default config template

## Design decisions
- Pause is implemented via `task.pause = true` (not direct `State::Pause` set) so the existing `apply_pause_to_state` flow properly pushes the stack entry for resumability
- The stage_count is already persisted in GitHub parameters (zbobr-task-backend-github) — no changes needed there
- No existing tests referenced `increment_stage_count` directly; all 96 tests pass unchanged

## Verification
- `cargo build`: success
- `cargo test`: 96 passed, 7 ignored (GitHub integration tests)