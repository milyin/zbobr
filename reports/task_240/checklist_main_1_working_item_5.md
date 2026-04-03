Add `select_runnable_task(workflow: &Workflow, tasks: &[Task]) -> Option<&Task>` to cli.rs that:
- Checks `!t.pause` (to handle tasks where pause flag is set but state not yet transitioned)
- Calls `workflow.resolve_next_action(t)` 
- Keeps only tasks with `StateAction::RunStage` where `stage_def.call_pipeline().is_none()` (non-call)
- Returns the highest-priority one via `task_priority`

Export it from lib.rs. Use it in commands.rs for --select (replacing select_ready_task). Also update loop Phase 2 to use `select_runnable_task(workflow, &runstage_candidates)` to truly share the same logic.