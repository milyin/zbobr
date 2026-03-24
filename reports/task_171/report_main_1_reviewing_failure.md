# Review Report: ERROR Section Implementation

## Summary
The implementation is incomplete. While the `Task` struct and `parse_description_full`/`serialize_description_full` logic have been updated, the crucial changes to persist the error field in the FS backend and to actually use the `ERROR` section instead of posting comments (via `stop_with_error`) are missing.

## Findings

### 1. Missing Backend Persistence (FS Backend)
- **File**: `zbobr-task-backend-fs/src/fs.rs`
- **Issue**: The `TaskFile` struct was not updated to include `error: Option<String>`.
- **Impact**: The error field will be lost when tasks are saved/loaded using the FS backend.
- **Fix**: Add `#[serde(default)] pub error: Option<String>` to `TaskFile`. Update `from_task` to copy `task.error` to `TaskFile.error`. Update `to_task` to copy `self.error` to `Task.error`.

### 2. Missing Logic Update (`stop_with_error`)
- **File**: `zbobr-dispatcher/src/mcp/traits.rs` (or `zbobr-dispatcher/src/task.rs` if moved)
- **Issue**: The implementation of `stop_with_error` still posts a comment instead of updating the task's error field.
- **Impact**: Errors will continue to appear as comments, violating the requirement "The error reports should be placed to dedicated section ERROR... instead of posting comment".
- **Fix**: Update `stop_with_error_impl` to call `self.session().set_error(Some(message))` instead of `post_comment`.

### 3. Missing Helper Method (`set_error`)
- **File**: `zbobr-dispatcher/src/task.rs` (RoleSession) or `zbobr-api/src/backend.rs` (TaskMut trait)
- **Issue**: `set_error` method is missing.
- **Impact**: No way to update the error field cleanly.
- **Fix**: Implement `set_error` in `RoleSession` (which calls `backend.mutate`) and/or add it to `TaskMut` trait if needed.

### 4. Tests Not Updated
- **File**: `zbobr-dispatcher/tests/mcp_integration/test_helpers.rs`
- **Issue**: Tests were not updated to assert on `task.error` instead of looking for error comments.
- **Impact**: Regression testing is incomplete; the new functionality is not verified by integration tests.
- **Fix**: Update assertions in `run_report_error_preserves_signal` and `run_exit_preserves_agent_set_signal`.

## Checklist Status
The following checklist items were marked as unchecked or were verified as incomplete:
- [ ] [id: backend-set-error] Add `set_error` to `TaskMut` trait.
- [ ] [id: fs-backend-update] Add `error` field to `TaskFile`.
- [ ] [id: role-session-set-error] Add `set_error` to `RoleSession`.
- [ ] [id: rewrite-stop-with-error] Rewrite `stop_with_error_impl`.
- [ ] [id: update-tests] Update test assertions.

## Recommendation
Complete the implementation by addressing the missing items listed above. Ensure the FS backend persists the error field and `stop_with_error` correctly updates the `ERROR` section. Verify with updated tests.