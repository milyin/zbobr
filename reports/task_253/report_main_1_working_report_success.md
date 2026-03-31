# Task #253 Complete: Simplify to Single-Repo Support

## Summary

All 6 checklist items completed and committed in a single commit (`a103bed`).

## Changes Made

### Core API (`zbobr-api`)
- **`task.rs`**: Simplified `TaskIdentity` from 4 fields to 2 (`task_id` + `work_branch`). Updated `Task::identity()` accordingly.
- **`backend.rs`**: Added `repository()`, `branch()`, `repo_name()` methods to `WorktreeBackend` trait.
- **`config.rs`**: Removed `default_destination_repository` and `default_destination_branch` from `ZbobrDispatcherConfig`.
- **`config_tools.rs`**: Removed `ConfigureWorktree` from `McpTool` enum and all related constants.

### Repo Backends
- **`zbobr-repo-backend-fs/config.rs`**: Added `repository` and `branch` fields; implemented `repo_short_name()`.
- **`zbobr-repo-backend-fs/fs.rs`**: Updated to use configured `repository`/`branch` instead of task identity fields; implemented new trait methods.
- **`zbobr-repo-backend-github/config.rs`**: Removed `fork_owner`; added `repository` and `branch`; implemented `repo_short_name()`.
- **`zbobr-repo-backend-github/github.rs`**: Removed all fork logic (`ensure_fork`, `ensure_fork_remote`, `sync_fork_base_with_upstream`, `push_worktree_to_remote`→`push_worktree_to_origin`). Simplified to always use "origin". Implemented new trait methods.

### Dispatcher (`zbobr-dispatcher`)
- **`lib.rs`**: Simplified `create_task()` and `create_task_with_confirm()` (removed dest_repo/dest_branch params). Updated `update_worktree()` to use `repo_backend.repo_name()`.
- **`cli.rs`**: Added `sanitize_branch_postfix()` and `ensure_work_branch()` for auto-deriving work branch from task title. Updated `detect_and_handle_worktree()` and `perform_stash_and_push()` to use backend config.
- **`mcp/common.rs`**: Removed `ConfigureWorktreeParam`.
- **`mcp/mod.rs`**: Removed `ConfigureWorktreeParam` from pub use.
- **`mcp/traits.rs`**: Removed `configure_worktree_impl()` and `configure_worktree_error()` (~120 lines).
- **`mcp/unified.rs`**: Removed `configure_worktree` tool definition.
- **`task.rs`** (tests): Added `repository()`, `branch()`, `repo_name()` to `DummyRepo`; fixed `create_task` call args.

### zbobr Binary
- **`init.rs`**: Removed preparator stage from `default_workflow()`, removed "preparator" role, removed `PREPARATOR_PROMPT` and `PREPARATOR_TASK_TEMPLATE` constants and their entries in `PROMPT_FILES`. Updated `default_config_toml()` to use `repository`/`branch` instead of `fork_owner`.
- **`commands.rs`**: Removed `dest_repo`/`dest_branch` from `TaskSubcommand::Create`. Updated `overwrite_author()` to use backend methods.

### Integration Tests
- **`env.rs`**: Removed `fork_owner` field; updated `init_fs_fs` to create shared test repo; updated `init_github_github` to use `repository`/`branch`; fixed `create_task` call.
- **`abstract_scenarios.rs`**: Removed `configure_worktree_*` scenario functions.
- **`abstract_test_helpers.rs`**: Removed `run_configure_worktree_*` test helper functions.
- **`integration_fs_fs.rs`**: Removed two configure_worktree tests.
- **`integration_github_github.rs`**: Removed configure_worktree tests; updated credentials loading.

### Sample Config
- **`zbobr_github_test.toml.sample`**: Updated `[repo]` section to use `repository`/`branch` instead of `fork_owner`.

## Test Results
- All unit tests pass (13/13 integration fs_fs, 39/39 dispatcher unit tests)
- Pre-existing test failure `compact_comment_uses_first_line_only` in zbobr-api is unrelated to this task