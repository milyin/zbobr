In zbobr-api/src/task.rs, `Task::identity()` was simplified: it now returns `Some(TaskIdentity)` when only `work_branch` is set (previously required `destination_repository` and `destination_branch` too). The `TaskIdentity` struct itself no longer has those fields.

Add tests in the existing `#[cfg(test)]` module in task.rs:
- `identity()` returns `Some` when `work_branch` is set
- `identity()` returns `None` when `work_branch` is `None`
- Verify returned `TaskIdentity` contains correct `task_id` and `work_branch`

This ensures the simplified contract is codified.