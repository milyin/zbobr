## Root Cause

In `zbobr-repo-backend-github/src/github.rs`, the `ensure_pr_url` method short-circuits when `find_existing_pr` succeeds:

```
if let Ok(url) = self.find_existing_pr(&pr_repo, work_branch, None).await {
    return Ok(url);   // ← returns without updating PR body
}
```

The PR was previously created by `ensure_pr_exists` (called during `update_worktree`) with an empty `"body": ""`. When `ensure_pr_url` is later called from `cli.rs` with a task link body (e.g. `"Resolves https://github.com/owner/repo/issues/42"`), it finds the existing PR and returns early — never updating the description.

## Proposed Fix

1. Modify `find_existing_pr` to return `(html_url, pr_number)` instead of just `html_url` — the PR number is needed to call the PATCH endpoint.
2. In `ensure_pr_url`, when finding an existing PR and `body` is `Some`, call `PATCH /repos/{pr_repo}/pulls/{pr_number}` to update the description before returning.

The FS backend's `ensure_pr_url` ignores `_body` and doesn't need changes (it's a local filesystem mock, not a real PR host).

This is a minimal, targeted fix: no changes to the trait interface, no changes to callers.
