All done. The task is complete:

- **Removed**: preparator stage, `configure_worktree` MCP tool, fork support from GitHub backend, per-task `destination_repository`/`destination_branch` routing config
- **Added**: auto-derivation of work branch from task title (`{prefix}-{task_id}-{sanitized-title}`), `repository`/`branch` fields on repo backend configs
- **Simplified**: `TaskIdentity` (4 fields → 2), `ZbobrRepoBackendGithub` (removed all fork logic), `ZbobrDispatcherConfig` (removed two fields)
- All 23 changed files committed in `a103bed`