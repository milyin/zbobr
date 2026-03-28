In `zbobr-dispatcher/src/cli.rs`, after the executor returns, save the captured output and link it to the stage — mirroring exactly how the prompt is saved before execution.

**`SessionOutcome`**: add an `output: String` field alongside `execution_interrupted` and `execution_error`.

**`execute_tool`**: update to propagate the returned `String` from `executor.execute(...)` into `SessionOutcome::output`. On error or interruption, use `String::new()` for the output field.

**After `execute_tool` returns** (in the `run_stage` method, just after the outcome is obtained and before `finalize_stage_session`): if `outcome.output` is non-empty, save it the same way the prompt is saved:
- `base_name = format!("output_{pipeline}_{run_id}_{stage}_end", ...)`
- Call `role_session.store_report(&base_name, &outcome.output).await?`
- Update `stage.info.output_link` via `modify_task` (same pattern as the prompt_link update)

Save even if execution failed, so output from a failed run is still accessible.