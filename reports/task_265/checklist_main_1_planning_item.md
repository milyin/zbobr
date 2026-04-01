In `zbobr-dispatcher/src/cli.rs`, function `process_task()`, the call at line 893:

```
runner.run().await?;
```

propagates any error out of `process_task`, leaving the task in `Running` state with the signal already cleared (signal was cleared on line 441). This means the task is stuck with no way to resume.

**What to change:**
Replace `runner.run().await?` with an error handler that mirrors the existing pattern used in worktree error recovery (see lines 1596-1607):

1. Catch the error: `if let Err(e) = runner.run().await { ... }`
2. Log the error with `tracing::error!`
3. Format the status using `format_error_status(zbobr, &msg)` (same helper used throughout cli.rs)
4. Call `task_session.set_pause_with_status_and_signal(status, Signal::go(stage_name.to_string()))` — use the `stage_name` that is already in scope from the `RunStage` match arm
5. If the pause call itself fails, log with `tracing::error!` but do not propagate (same fallback pattern as lines 1601-1607)
6. Return `Ok(())` — the error has been handled gracefully

**Why this works:** After this call, `task.pause == true` and `task.signal == Some(Go(stage_name))`. The state is still `Running(pipeline, stage)`. On the next dispatcher iteration, `apply_pause_to_state` reads `task.state.pipeline()` (which returns the pipeline from `Running(p, _)`), pushes a `StackEntry`, and sets state to `Pause`.
