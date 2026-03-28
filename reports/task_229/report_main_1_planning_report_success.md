# Plan: Fix PR description missing task link

## Root Cause

Two-part bug in `zbobr-repo-backend-github/src/github.rs`:

1. `update_worktree` calls the private `ensure_pr_exists` which creates the PR with `"body": ""` (empty)
2. `ensure_pr_url` (the public trait method) then finds the existing PR and returns the URL immediately without updating the body — even when `body: Option<&str>` is `Some(task_link)`

The result: PRs are always created with empty descriptions.

## Fix

**Checklist item 1**: Modify `ensure_pr_url` in github.rs to PATCH the PR body when the PR already exists and `body` is `Some(...)`. Requires returning the PR number from `find_existing_pr` alongside the URL.

**Checklist item 2**: Remove `ensure_pr_exists` calls from `update_worktree` (Phase 5). PR creation becomes the sole responsibility of `ensure_pr_url`, which is already called by the prepare stage right after `update_worktree` with the correct task-link body. This eliminates the duplication and ensures the PR is always created with the correct body.

## Why this approach

Per user instruction: refactor to avoid duplication, make body optional (update if passed), have configure-workspace use the same PR creation path as others. The prepare stage already has the right call sequence — just need to stop `update_worktree` from creating PRs with empty body, and make `ensure_pr_url` update the body when a PR already exists.