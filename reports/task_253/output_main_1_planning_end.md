The plan is ready for review. Here's a summary:

## Proposed implementation plan

**Core idea**: repo backends become single-repo, routing fields are auto-populated at task creation, preparator stage is removed entirely.

### Changes:
1. **Repo backend configs**: Add required `repository` + `branch` fields to both FS and GitHub backends; remove `fork_owner` from GitHub backend
2. **Dispatcher config**: Remove `default_destination_repository` and `default_destination_branch` (moved to repo backends)
3. **`WorktreeBackend` trait**: Add `repository()` and `branch()` methods so the dispatcher can read them
4. **Auto-populate task fields at creation**: `destination_repository`, `destination_branch` come from the backend config; `work_branch` is generated as `{prefix}-{id}-{slugify(title)}`
5. **GitHub backend**: Remove all fork infrastructure (`ensure_fork`, `ensure_fork_remote`, `sync_fork_base_with_upstream`) — always push to origin
6. **Remove `ConfigureWorktree` MCP tool**: from enum, trait impl, unified handler, and prompt variables
7. **Remove preparator stage** from default workflow in `init.rs` (stage, role, prompts, constants)
8. **Update integration tests**: remove/replace `configure_worktree` scenario tests

Please review and approve to proceed with implementation.