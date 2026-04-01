# Fix: Remove destination_repository/destination_branch from Task struct

## Problem
The review (ctx_rec_28) found that single-repo simplification was incomplete because `destination_repository` and `destination_branch` remained as fields on the `Task` struct, making per-task routing still possible and keeping these task-level fields authoritative.

## Changes Made (commit 93378bb)

### zbobr-api/src/task.rs
- Removed `pub destination_repository: Option<String>` and `pub destination_branch: Option<String>` from `Task` struct
- Updated test helper `make_task()` to remove these fields

### zbobr-api/src/backend.rs
- Removed `set_destination_repository()` and `set_destination_branch()` default methods from `TaskMut` trait

### zbobr-task-backend-github/src/github.rs
- Removed `PARAM_DESTINATION_REPOSITORY` and `PARAM_DESTINATION_BRANCH` constants
- Removed reading these params in `issue_to_task()`
- Removed writing them in `task_to_string_params()`
- Updated test helper Task literal

### zbobr-task-backend-fs/src/fs.rs
- Removed `destination_repository` and `destination_branch` from `TaskFile` internal struct
- Removed from `to_task()` and `from_task()` mapping
- Updated task creation literal

### zbobr-dispatcher/src/task.rs
- Removed `get_destination_repository()`, `set_destination_repository()`, `get_destination_branch()`, `set_destination_branch()` methods from `TaskSession`
- Fixed `finish()` to use `self.zbobr.repo_backend().repo_name()` instead of `task.destination_repository` for determining the work directory
- Updated test helper Task literal

### zbobr-dispatcher/src/cli.rs
- Removed setting `destination_repository`/`destination_branch` in `ensure_work_branch()`
- Removed displaying them in `print_task()`
- Updated comment on `ensure_work_branch`

### zbobr-dispatcher/src/prompts.rs
- Removed adding destination vars from task fields in `build_template_variables()` — these vars are still populated via `ConfiguredPromptBuilder::with_var()` from the repo backend in commands.rs
- Updated tests to remove destination field references

### zbobr-dispatcher/src/workflow.rs
- Updated test Task literal

### zbobr-dispatcher/tests/mcp_integration/env.rs
- Updated `update_task_branches()` to ignore `dest_repo`/`dest_branch` params (kept signature for callers, prefixed with `_`)

### zbobr/src/commands.rs
- Updated dummy task literal

## Result
- Clean build, no warnings
- 46 tests pass, 1 pre-existing unrelated failure (`compact_comment_uses_first_line_only`)
- The repo backend config is now the single source of truth for repository and branch routing