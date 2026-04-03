## Fix applied

The reviewer identified that `task list --select` used a weaker readiness predicate (state flags only) than the manager loop (which uses `workflow.resolve_next_action` to filter to `StateAction::RunStage` non-call tasks).

### Changes made

**`zbobr-dispatcher/src/cli.rs`**:
- Removed `select_ready_task` (state-flag based)
- Added `select_runnable_task<'a>(workflow: &Workflow, tasks: &'a [Task]) -> Option<&'a Task>` that:
  - Checks `!t.pause` (to handle tasks where pause flag is set but state not yet transitioned)
  - Calls `workflow.resolve_next_action(t)` for each task
  - Keeps only tasks returning `StateAction::RunStage` with `stage_def.call_pipeline().is_none()` (non-call stages that compete for a slot)
  - Returns the highest-priority one via `task_priority`
- Updated loop Phase 2 to call `select_runnable_task(workflow, &runstage_candidates)` instead of `select_ready_task(&runstage_candidates)` — truly shared logic

**`zbobr-dispatcher/src/lib.rs`**: exports `select_runnable_task` instead of `select_ready_task`

**`zbobr/src/commands.rs`**: `--select` path now calls `select_runnable_task(zbobr.workflow(), &tasks)` instead of `select_ready_task(&tasks)`

### Result
- `task list --select` and the loop Phase 2 now use the exact same function with the exact same predicate
- A task only appears in `--select` output if the workflow engine would actually run it in Phase 2
- Build passes cleanly; pre-existing test failures are unrelated (rustls CryptoProvider issue)