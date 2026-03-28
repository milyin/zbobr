Plan finalized. Here's what will be implemented:

**Item 1 — Fix `ensure_pr_url` to update existing PR body**: When `body` is `Some(...)` and a PR already exists, call `PATCH /repos/{owner}/{repo}/pulls/{number}` to set the description before returning the URL. Requires returning the PR number from `find_existing_pr`.

**Item 2 — Remove PR creation from `update_worktree`**: Delete the `ensure_pr_exists` calls from Phase 5 of `update_worktree` and remove the `ensure_pr_exists` method entirely. The prepare stage already calls `ensure_pr_url` (with the task-link body) right after `update_worktree`, so PR creation coverage is maintained and PRs will be created with the correct body from the start.