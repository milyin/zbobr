# Implementation: Add issue link to PR body

## Changes (6 files, +32/-5 lines)

### 1. `zbobr-api/src/backend.rs`
- Added `task_repo_name()` default method to `TaskBackend` trait (returns `None` by default)
- Added `body: Option<&str>` parameter to `WorktreeBackend::ensure_pr_url()`

### 2. `zbobr-task-backend-github/src/github.rs`
- Overrode `task_repo_name()` in `TaskBackendGithub` to return `Some(self.inner.backend_config.github_repo.clone())`

### 3. `zbobr-repo-backend-github/src/github.rs`
- Updated `ensure_pr_url` impl to accept `body: Option<&str>` and use it in the PR creation payload via `body.unwrap_or("")`

### 4. `zbobr-repo-backend-fs/src/fs.rs`
- Updated `ensure_pr_url` signature to accept `_body: Option<&str>` (unused in fs backend)

### 5. `zbobr-dispatcher/src/task.rs`
- Updated mock `DummyRepo::ensure_pr_url` signature to accept `_body: Option<&str>`

### 6. `zbobr-dispatcher/src/cli.rs`
- In `ensure_pr_url()`, constructs issue URL from `task_backend().task_repo_name()` and `task_id`
- Passes the URL as body to `repo_backend().ensure_pr_url()` so new PRs contain "Resolves https://github.com/{repo}/issues/{task_id}"

## Verification
- `cargo build` — compiles successfully
- `cargo test` — all 105 tests pass, 0 failures
