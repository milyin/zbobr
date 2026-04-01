Two compilation errors in zbobr/src/commands.rs:

1. `VAR_DESTINATION_BRANCH` and `VAR_DESTINATION_REPOSITORY` are not re-exported from zbobr_dispatcher (they exist in prompts.rs but lib.rs doesn't re-export them).

2. `Task` struct initialization in `dummy_task_and_comments()` is missing `destination_branch` and `destination_repository` fields that still exist in zbobr-api/src/task.rs.

Fix:
- Add `VAR_DESTINATION_BRANCH, VAR_DESTINATION_REPOSITORY` to pub use prompts:: in zbobr-dispatcher/src/lib.rs
- Add `destination_branch: None, destination_repository: None` to Task initializer in commands.rs