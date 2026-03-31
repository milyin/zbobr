# Planning Complete: Simplify to Single Repository

## Approach

The simplification has four main pillars:

1. **Remove preparator stage** — The preparator's only meaningful job was providing a branch name postfix. With auto-derivation from task title, it becomes unnecessary. The planner becomes the first stage.

2. **Auto-derive branch names** — The `work_branch_postfix` previously came from the preparator agent. Now it's slugified from the task title in `configure_worktree_impl`. Format stays: `{prefix}-{task_id}-{slug}`.

3. **Move repo/branch config to backend** — `default_destination_repository` and `default_destination_branch` move from `ZbobrDispatcherConfig` to both repo backend configs as required `repository` and `branch` fields. The dispatcher no longer needs to know what repo it's working on.

4. **Remove fork support** — GitHub backend loses `fork_owner`, `ensure_fork()`, `ensure_fork_remote()`, and the "fork" remote. Work branches are pushed directly to origin. The FS backend serves as the analog for the simplified shape.

## Key Design Decisions

- **Analog**: The filesystem backend (`zbobr-repo-backend-fs`) is the model for the simplified GitHub backend — no fork, direct bare clone, single remote.
- **TaskIdentity simplification** is a consequence of the above: once the backend owns repo/branch config, passing them through TaskIdentity is redundant.
- **No backward compatibility layer** — these are clean breaking changes, consistent with the task's intent to simplify rather than maintain flexibility.

## Checklist Items Created
1. Remove preparator stage from default workflow
2. Derive work branch name automatically from task name
3. Move repository/branch from dispatcher config to repo backend config
4. Remove fork support from GitHub repo backend
5. Simplify TaskIdentity and repo backend trait interface
6. Update tests, config examples, and documentation