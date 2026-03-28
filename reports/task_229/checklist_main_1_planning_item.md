## What

In `zbobr-repo-backend-github/src/github.rs`, the `ensure_pr_url` method (around line 968) calls `find_existing_pr` and when the PR exists, immediately returns the URL without setting the body. If `body` is `Some(...)`, it should also update the PR description via the GitHub PATCH API (`PATCH /repos/{owner}/{repo}/pulls/{pull_number}`).

## Why

When the prepare stage runs, `update_worktree` creates the PR first with empty body (via `ensure_pr_exists`, line 513: `"body": ""`), and then the dispatcher calls `backend.ensure_pr_url` with the task-link body. But `ensure_pr_url` finds the existing PR and returns early, so the body is never set.

## How to implement

- Modify `find_existing_pr` to also return the PR number (add `number: u64` to the local `PrListItem` struct, change return type to include number alongside URL)
- In `ensure_pr_url`: after the `find_existing_pr` succeeds, if `body` is `Some(...)`, call `PATCH /repos/{pr_repo}/pulls/{pr_number}` with `{"body": ...}` via `self.octocrab` before returning the URL
- Follow the same retry/error handling patterns already used in the file

## Analog

The `ensure_pr_exists` method in the same file shows how to call the octocrab PATCH-style endpoint. The `find_existing_pr` return-type change is a local refactor within the private impl block.