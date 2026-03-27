# Write pipeline, stage, signal to PARAMETERS section

**File:** `zbobr-task-backend-github/src/github.rs`
**Function:** `task_to_string_params()` (lines ~799–837)

Add three new entries to the params HashMap:

1. **`pipeline`**: Extract from the task's state if it's `Pending(pipeline)` or `Running(pipeline, _)`. Use `pipeline.as_str()`.
2. **`stage`**: Extract from the task's state if it's `Running(_, stage)`. Use `stage.as_str()`.
3. **`signal`**: If `task.signal` is `Some(sig)`, format it as `format!("{sig}")` (Signal implements Display).

Example additions after the existing entries:

```rust
// Write pipeline and stage from state
match &task.state {
    State::Pending(pipeline) => {
        params.insert("pipeline".to_string(), pipeline.as_str().to_string());
    }
    State::Running(pipeline, stage) => {
        params.insert("pipeline".to_string(), pipeline.as_str().to_string());
        params.insert("stage".to_string(), stage.as_str().to_string());
    }
    _ => {}
}
// Write signal
if let Some(ref sig) = task.signal {
    params.insert("signal".to_string(), format!("{sig}"));
}
```

**Why:** These values will now be persisted in the issue body PARAMETERS section instead of as labels, following the same pattern as `pipeline_run_id`.