# Completed: update-backends

## Changes Made

### GitHub backend (zbobr-task-backend-github/src/github.rs)
- `issue_to_task` signature changed from `-> Task` to `-> anyhow::Result<Task>` since `parse_description_full` now returns `Result`
- 4th destructured field renamed from `checklist` to `context` (which is now `TaskContext`)
- Task struct construction uses `checklist: vec![], context,` instead of `checklist,`
- All `serialize_description_full` calls changed from `&task.checklist` to `&task.context`
- All `serialize_description_full` calls for `current_task` changed similarly
- `merge_concurrent_description_updates` call now unwraps with `?`
- `create_task` uses `&TaskContext::default()` instead of `&[]`
- All callers of `issue_to_task` updated to handle `Result` with `?`
- Added `TaskContext` to imports

### FS backend (zbobr-task-backend-fs/src/fs.rs)
- `TaskFile` struct: replaced `checklist: Vec<ChecklistItem>` with `context: TaskContext` (with `#[serde(default)]`)
- `to_task()`: maps `context: self.context.clone()`, sets `checklist: vec![]`
- `from_task()`: maps `context: task.context.clone()` instead of `checklist: task.checklist.clone()`
- `create_task()`: Task construction includes `context: TaskContext::default()`
- Updated imports: removed `ChecklistItem`, added `task::TaskContext`
- Fixed test module import for `ChecklistItem` in checklist_format_tests

### Other files updated (test helpers / dummy_task functions)
- `zbobr-dispatcher/src/prompts.rs`: added `context: TaskContext::default()` to dummy_task
- `zbobr-dispatcher/src/task.rs`: added `context: TaskContext::default()` to test Task construction
- `zbobr-dispatcher/src/workflow.rs`: added `context: TaskContext::default()` to test Task construction, added `TaskContext` import
- `zbobr/src/commands.rs`: added `context: TaskContext::default()` to dummy_task_and_comments

## Verification
- `cargo build` succeeds with no errors
- `cargo test` passes all 123 tests with 0 failures