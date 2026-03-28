## What

In `zbobr-repo-backend-github/src/github.rs`, Phase 5 of `update_worktree` calls `ensure_pr_exists` in both branches (whether `!remote_exists` or not). Remove these calls. PR creation becomes the sole responsibility of `ensure_pr_url`. Once the calls are removed, `ensure_pr_exists` itself can be deleted as it will be unused.

## Why

This eliminates the root cause of the duplication: `update_worktree` creates the PR with an empty body, then `ensure_pr_url` finds it already exists and (before the fix in item 1) returns without setting the body. By removing PR creation from `update_worktree`, the PR is always created by `ensure_pr_url` which already has the task-link body available. The prepare stage already calls `ensure_pr_url` right after `detect_and_handle_worktree` (which calls `update_worktree`), so PR creation coverage is maintained.

## How to implement

- Remove the two `self.ensure_pr_exists(...)` call sites in `update_worktree` (Phase 5, lines ~897-902)
- Delete the `ensure_pr_exists` private async method (lines ~503-542) since it will have no callers
- The `update_worktree` method should still push the branch to remote (the push in Phase 5 stays); only the PR creation calls are removed

## Impact on other callers

`perform_stash_and_push` in cli.rs calls `update_worktree` but does not call `ensure_pr_url` — this is fine because the PR was already created during the prepare stage which runs before any worker stage. The prepare stage is the only place that calls `ensure_pr_url` with a body, so PRs will be created with the task link on first setup.