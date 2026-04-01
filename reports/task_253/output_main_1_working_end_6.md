The fix is complete. The core issue was that `destination_repository` and `destination_branch` were still stored as fields on the `Task` struct, which meant per-task routing was still possible and these task-level fields remained authoritative — contradicting the single-repo simplification goal.

**Changes made:**
- Removed `destination_repository` and `destination_branch` fields from `Task` struct in `zbobr-api`
- Removed `set_destination_*` methods from `TaskMut` trait
- Removed from GitHub backend serialization/deserialization
- Removed from FS backend internal `TaskFile` struct
- Removed getter/setter methods from `TaskSession`
- Fixed `finish()` to use `repo_backend.repo_name()` for work dir path
- Cleaned up `ensure_work_branch`, `print_task`, test helpers, and integration test env

The repo backend config is now the sole source of truth for routing. Template variables `{destination_repository}` and `{destination_branch}` are still populated in prompts via `with_var()` from the backend — not from task fields.