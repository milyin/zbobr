# Fix: PR description missing task link

## Root Cause
`ensure_pr_url` in `github.rs` called `find_existing_pr` and returned early on success, skipping the body update with the task link. The `update_worktree` function also created PRs via `ensure_pr_exists` with an empty body before `ensure_pr_url` was called, making the early return always trigger.

## Changes (zbobr-repo-backend-github/src/github.rs)

1. **Added `ExistingPr` struct** holding `html_url` and `number` (PR number needed for PATCH).

2. **Updated `find_existing_pr`** to return `ExistingPr` instead of just `String`, including the PR number.

3. **Added `update_pr_body` helper** that PATCHes `/repos/{pr_repo}/pulls/{pr_number}` with the new body.

4. **Fixed `ensure_pr_url`**: when an existing PR is found and `body` is `Some`, now calls `update_pr_body` before returning. Same fix applied to the 422 fallback path.

5. **Removed `ensure_pr_exists`** from `update_worktree` (both the new-branch and existing-branch paths). PR creation is now solely handled by `ensure_pr_url`. Also removed the now-unused `ensure_pr_exists` method entirely.

## Result
When `configure_workspace` calls `ensure_pr_url` with the task link body, the PR body is always set correctly — whether the PR is newly created or already existed.
