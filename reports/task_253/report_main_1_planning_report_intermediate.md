# Implementation Plan: Simplify to Single-Repository

## Overview

Remove the preparator stage and centralize repository configuration in the repo backends. Task routing fields are auto-populated at task creation; work branch is derived from task title in code.

## Changes by component

### 1. Add `repository` and `branch` to repo backend configs
- **`zbobr-repo-backend-fs/src/config.rs`**: Add required `repository: String` (local path) and `branch: String` (base branch name)
- **`zbobr-repo-backend-github/src/config.rs`**: Add required `repository: String` (owner/repo) and `branch: String` (base branch). Remove `fork_owner: String`.

### 2. Remove dispatcher config fields
- **`zbobr-api/src/config.rs`**: Remove `default_destination_repository: Option<String>` and `default_destination_branch: Option<String>` from `ZbobrDispatcherConfig` and its `Default` impl/TOML/Args structs.

### 3. Expose repository/branch on `WorktreeBackend` trait
- **`zbobr-api/src/backend.rs`**: Add `fn repository(&self) -> &str` and `fn branch(&self) -> &str` to `WorktreeBackend` trait. Both FS and GitHub backends implement using their config fields.

### 4. Auto-populate task routing fields at creation
- **`zbobr-dispatcher/src/lib.rs`**: In `create_task_with_confirm`, read `repository` and `branch` from `self.repo_backend` and auto-set `task.destination_repository`, `task.destination_branch`, and `task.work_branch`. Work branch is generated as `{work_branch_prefix}-{task_id}-{slugify(title)}`.
- Add a `slugify(title)` helper (lowercase, spaces → hyphens, drop non-alphanumeric-or-hyphen, truncate to ~40 chars).

### 5. Simplify GitHub backend — remove fork logic
- **`zbobr-repo-backend-github/src/github.rs`**: Remove `ensure_fork`, `ensure_fork_remote`, `sync_fork_base_with_upstream`. In `update_worktree`, drop the `same_org` / cross-org branch: always push to "origin", use backend's configured `repository`.

### 6. Remove `ConfigureWorktree` MCP tool
- **`zbobr-api/src/config_tools.rs`**: Remove `ConfigureWorktree` variant from `McpTool` enum and `ALL_TOOLS` / `ALL_TOOL_NAMES`.
- **`zbobr-dispatcher/src/mcp/traits.rs`**: Remove `configure_worktree_impl` and `configure_worktree_error` methods.
- **`zbobr-dispatcher/src/mcp/unified.rs`**: Remove `configure_worktree` MCP handler.
- **`zbobr-dispatcher/src/prompts.rs`**: Remove `mcp_configure_worktree` prompt template variable.

### 7. Remove preparator stage from default workflow
- **`zbobr/src/init.rs`**: Remove "preparing" stage from `main_stages`, remove "preparator" role definition, remove `PREPARATOR_PROMPT` and `PREPARATOR_TASK_TEMPLATE` constants, remove `preparator_task.md` from `PROMPT_FILES`, remove `preparator_task_prompt` variable.
- Update `default_config_toml()` to remove `default_destination_repository`, `default_destination_branch` from dispatcher section, remove `fork_owner` from repo section, add `repository` and `branch` to repo section.

### 8. Update integration tests
- **`zbobr-dispatcher/tests/`**: Update/remove tests for `configure_worktree_idempotent` and `configure_worktree_ignore_requested_postfix` — replace with tests verifying auto-populated routing fields at task creation.
- **`tests/mcp_integration/abstract_scenarios.rs`**: Remove or replace `configure_worktree_*` scenario functions.

## Key design decisions
- The `WorktreeBackend` trait grows two new required methods (`repository`, `branch`) so the dispatcher can read them without knowing the concrete type. Both backends implement them from their config.
- Work branch slug: lowercase title, replace spaces and non-alphanumeric chars with hyphens, collapse multiple hyphens, truncate to 40 chars.
- GitHub backend becomes same-repo-always: no fork, no cross-org. The entire fork infrastructure is deleted.
- `TaskIdentity` struct is unchanged — `destination_repository` and `destination_branch` in it will now always reflect the backend's configured values (set at creation).

## Analog
The closest analog for the repo backend config change is `ZbobrTaskBackendGithubConfig` which already has a `github_repo: String` required field representing the single tracked repository. The new `repository` / `branch` fields in repo backends follow the same pattern.