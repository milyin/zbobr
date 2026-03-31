In `zbobr-dispatcher/src/mcp/traits.rs`, the `configure_worktree_impl` function currently takes a `work_branch_postfix` parameter supplied by the preparator agent. Since the preparator is removed, the work branch postfix should now be derived automatically from the task's short name/title.

The branch name format stays: `{work_branch_prefix}-{task_id}-{slugified_task_name}`.

The slugification should convert the task title to a lowercase hyphen-separated string safe for use as a branch name component (strip punctuation, replace spaces with hyphens, truncate if too long).

The existing `create_branch_name` method in `task.rs` is the analog — the change is only in *where the postfix comes from* (task data instead of agent input). The task title is available from the task identity/metadata already fetched.

Why: Without a preparator agent, branch names must be derived programmatically. Using the task title is the natural source and keeps branch names meaningful.