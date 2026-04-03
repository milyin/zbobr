## Summary

Fixed the two issues identified in the latest review (ctx_rec_12):

### 1. Restored `title` field in `TaskListEntry`
- `zbobr-dispatcher/src/cli.rs`: renamed `description: String` → `title: String` in `TaskListEntry` struct and its `From<&Task>` impl (now populates from `task.title`)
- `zbobr/src/commands.rs`: compact list output now prints `task.title` instead of `task.description`

### 2. Wired `select_ready_task` into `run_manager_loop`
Refactored the single-pass loop into a two-phase approach:
- **Phase 1**: iterates all tasks, applies state transitions (pause, ready-from-state), handles `Done` actions eagerly (for all tasks), handles instant call-stage actions eagerly. Non-call `RunStage` tasks are collected into `runstage_candidates`.
- **Phase 2**: calls `select_ready_task(&runstage_candidates)` to pick the highest-priority ready task, then runs its stage.

This shares the exact same ready-task selection logic between `task list --select` and the manager loop.

### Build/Test
- `cargo build`: ✅ passes
- `cargo clippy`: ✅ no new warnings
- `cargo test`: 3 pre-existing failures in `zbobr-task-backend-github` (rustls CryptoProvider env issue, unrelated to these changes)