In `zbobr-dispatcher/src/cli.rs`, function `run_manager_loop()`, the error block at lines 1120-1127:

```rust
if let Err(e) = runner.run().await {
    let msg = format!("Stage {}/{} failed for task #{}: {e}", pipeline_name, stage_name, task.id);
    tracing::error!("{msg}");
    set_task_status_with_log(zbobr, task.id, "stage run failure", &msg).await;
}
```

Only logs and sets a status string but does NOT pause the task or restore its state for retry.

**What to change:**
Keep `tracing::error!` but replace `set_task_status_with_log` with the proper graceful pause pattern:

1. Format: `let status = format_error_status(zbobr, &msg);`
2. Get session: `let task_session = zbobr.task_session(task.id);`
3. Call: `task_session.set_pause_with_status_and_signal(status, Signal::go(stage_name.to_string())).await`
4. If the pause call itself errors, log with `tracing::error!` but don't propagate (same fallback style as used elsewhere in the loop)

The `stage_name` variable is already in scope in this block (from the `RunStage` match arm at the surrounding loop). Use it directly for `Signal::go(stage_name.to_string())`.

**Why this works:** Same reasoning as Fix 1. The task was set to `Running(pipeline, stage)` before `runner.run()` was called (inside `CliStageRunner::run()`). After calling `set_pause_with_status_and_signal`, `apply_pause_to_state` at the top of the next loop iteration will atomically push the context and set state to `Pause`.
