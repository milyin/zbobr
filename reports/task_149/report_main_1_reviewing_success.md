# Review Report: Add Stage Counting

## Summary
The implementation adds a `stage_count` field to the `Task` struct and automatically increments it on every stage execution and pipeline call. The changes are spread across the API, dispatcher, and backends (FS and GitHub).

## Findings

### Correctness
- **Data Model**: `stage_count: u64` added to `Task` in `zbobr-api/src/task.rs` with `#[serde(default)]` for backward compatibility.
- **Logic**: `increment_stage_count` method added to `TaskSession` in `zbobr-dispatcher/src/task.rs`.
- **Integration**: The increment method is correctly called in `zbobr-dispatcher/src/cli.rs` in two critical places:
    1. `process_task` (or `run` loop): Ensures count increments when entering a standard stage.
    2. `handle_call_stage`: Ensures count increments when a stage calls a sub-pipeline.
- **Persistence**: 
    - **FS Backend**: Field added to `FsTask` struct, ensuring it's saved to `task.json`.
    - **GitHub Backend**: Logic added to read/write `stage_count` from/to the issue body metadata (`params_map`), ensuring persistence in GitHub issues.

### Consistency
- The implementation follows the existing pattern of using `modify_task` with a closure for atomic-like updates to the task state.
- Struct initialization was updated across the codebase.
- The field is handled similarly to `pipeline_run_id` and other metadata.

### Observations
- **Visibility**: The `stage_count` is *not* displayed in the `print_task` output (CLI view). This means users won't see this counter when inspecting a task. While not explicitly requested, it might be useful for debugging.
- **Prompts**: The `stage_count` is not currently exposed to the agent via the prompt. This is consistent with the requirement which only asked for the parameter to be added and incremented, presumably for tracking or future logic usage.

### Conclusion
The changes fully meet the task requirements ("add numeric parameter to task which is automatically increased on each stage passed") and follow the project's architectural patterns.

**Status**: Approved
