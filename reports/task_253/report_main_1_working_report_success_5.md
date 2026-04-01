# Fix Summary

Fixed all 3 issues identified in review ctx_rec_43:

## Issue 1: GitHub repo normalization (commit 02e3dec)
**Problem:** `ensure_pr_url`, `validate_connectivity`, and other methods in `ZbobrRepoBackendGithub` used `backend_config.repository` directly in GitHub API endpoint URLs (e.g. `/repos/{repository}/pulls`). If the user configured `repository` as an HTTPS URL (`https://github.com/owner/repo`) or SSH URL (`git@github.com:owner/repo`), the API calls would fail because the endpoint would include the full URL instead of just `owner/repo`.

**Fix:** In `ZbobrRepoBackendGithub::from_config()` (`zbobr-repo-backend-github/src/github.rs`), after calling `validate()`, parse the repository with `parse_github_repo()` and store the normalized `full_name` (`owner/repo`) back into `backend_config.repository`. All downstream code then sees a consistently normalized value.

## Issue 2: Wrong GitHub test target wiring (commit 02e3dec)
**Problem:** In `init_github_github()` (`zbobr-dispatcher/tests/mcp_integration/env.rs`), `target_repo` was set to `Some(task_repo)` (the task tracking repository) instead of `Some(repository)` (the repo backend repository that agents work on).

**Fix:** Changed `target_repo: Some(task_repo)` to `target_repo: Some(repository)`.

## Issue 3: Outdated docs (commit 02e3dec)
**Problem:** `README.md` line 163 still described the task project's `zbobr.toml` as `# zbobr configuration (fork owner, default model)`, referencing the removed fork owner concept.

**Fix:** Changed to `# zbobr configuration (default model, stage settings)`.

## Test results
All 46 tests pass. 1 pre-existing unrelated failure (`compact_comment_uses_first_line_only` in `zbobr-api`) remains.