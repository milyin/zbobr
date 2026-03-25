# Review Report: Task Priority & Counter Limit

## Assessment
The implementation correctly handles the priority requirement and configuration changes. However, there is a **critical bug** in the auto-pause logic within `handle_call_stage` and a logical issue in `CliStageRunner`.

## Critical Findings

### 1. Stack Duplication in `handle_call_stage`
In `zbobr-dispatcher/src/cli.rs`, the `handle_call_stage` function pushes a return frame to the stack *before* checking the `max_stage_count` limit.

```rust
// zbobr-dispatcher/src/cli.rs

task_session
    .push_stack(pipeline_name.clone(), return_signal.clone())
    .await?;
// ...
task_session.increment_stage_count().await?;

// Auto-pause if stage count limit reached.
{
    // ...
    if task.max_stage_count > 0 && task.stage_count >= task.max_stage_count {
        // ... sets pause=true and returns Ok(())
    }
}
```

**Impact:**
If the task is paused here, the stack already contains the return frame. When the task is resumed (unpaused), the `Call` stage is re-executed (retried). This calls `handle_call_stage` again, which **pushes the stack again**.
This leads to duplicate stack entries every time the task hits the limit and is resumed, potentially corrupting the return flow.

**Fix:**
Move the auto-pause check to the very beginning of `handle_call_stage`, *before* `push_stack`, `allocate_pipeline_run_id`, or `increment_stage_count`.

### 2. Premature Counter Increment in `CliStageRunner::run`
Similarly, in `CliStageRunner::run`, `increment_stage_count` is called before the auto-pause check.

```rust
// zbobr-dispatcher/src/cli.rs around line 353
task_session.increment_stage_count().await?;

// Auto-pause if stage count limit reached.
```

**Impact:**
1. The counter is incremented even if the task is immediately paused and does no work.
2. When resumed, the method runs again, incrementing the counter *again*.
This effectively "double charges" the task for the attempt, which might cause it to hit the limit again immediately if the user only increased the limit by 1.

**Fix:**
Move the auto-pause check to the beginning of `CliStageRunner::run` (or at least before `increment_stage_count`).

## Other Observations
- Priority sorting (`all_tasks.sort_by`) is implemented correctly.
- Configuration and task struct changes are correct.
- Handling of `0` as "no limit" is correct.
- Backwards compatibility for existing tasks (defaulting to 0) is preserved.

## Recommendation
Move the auto-pause logic blocks to the top of `handle_call_stage` and `CliStageRunner::run` to avoid side effects before the check.
