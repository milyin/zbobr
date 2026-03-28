All done. Here's a summary of the fix:

**Root cause**: `ensure_pr_url` returned early when a PR already existed (found via `find_existing_pr`), skipping the body update. Additionally, `update_worktree` pre-created PRs with an empty body via `ensure_pr_exists`, so `ensure_pr_url` always hit the early-return path.

**Changes in `zbobr-repo-backend-github/src/github.rs`**:
- Added `ExistingPr { html_url, number }` struct so `find_existing_pr` returns the PR number
- Added `update_pr_body` helper that PATCHes the PR via GitHub API
- Fixed `ensure_pr_url` to call `update_pr_body` when `body` is `Some` and PR exists (both the initial lookup and 422 fallback paths)
- Removed `ensure_pr_exists` calls from `update_worktree` — PR creation/body management is now solely in `ensure_pr_url`