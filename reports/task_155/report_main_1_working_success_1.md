# Work Report: Fix auto-pause ordering and deduplicate pause code

## Commit
`8c8f0be` — fix: reorder auto-pause checks and deduplicate pause code

## Changes

### 1. `zbobr-dispatcher/src/cli.rs`

**handle_call_stage (~line 609):** Reordered to: allocate_pipeline_run_id → increment_stage_count → auto-pause check (with early return) → push_stack. Previously push_stack came first, causing stack duplication on resume.

**CliStageRunner entry (~line 350):** Reordered to: auto-pause check (with early return) → increment_stage_count. Previously increment happened before the check, wasting a counter tick.

**Deduplication:** Replaced all 7 inline `modify_task(|mut t| { t.pause = true; ... })` patterns with:
- `task_session.set_pause(true)` for simple pause (2 locations: auto-pause in run_stage and handle_call_stage)
- `task_session.set_pause_with_signal(...)` for pause+signal (5 locations: finalize_stage_session failure, stash/push failure, finalize root pipeline failure, executor timeout root failure, merge conflict, SequentialSignal::Pause, SequentialSignal::PauseThenSignal)

### 2. `zbobr-dispatcher/src/task.rs`

Added `set_pause` and `set_pause_with_signal` helper methods on `TaskSession` (in addition to the existing `set_pause` on `RoleSession`). Added `set_pause_with_signal` on `RoleSession`.

## Verification
- `cargo build` — passes
- `cargo test` — all 105 tests pass
- `grep` for `.pause = true` in cli.rs — zero results (only remaining in task.rs helper implementations and set_state confirm logic)
