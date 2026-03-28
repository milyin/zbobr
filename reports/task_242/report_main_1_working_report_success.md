# Task 242: add allowed_usernames filter — completed

## Changes

### `zbobr-task-backend-github/src/config.rs`
- Added `allowed_usernames: Option<Vec<String>>` field to `ZbobrTaskBackendGithubConfig`

### `zbobr-task-backend-github/src/github.rs`
- Added `IssueUser { login: String }` struct
- Added `user: Option<IssueUser>` field to `IssueResponse` (populated from GitHub API)
- In `list_tasks`: if `allowed_usernames` is set, issues whose author login is not in the list are skipped
- Fixed test helper `make_issue_with_params` to include new `user: None` field

### `zbobr/src/init.rs`
- Added `allowed_usernames: None` to the default config template

## Verification
- `cargo build` — success
- `cargo test -p zbobr-task-backend-github` — 18 tests passed, 0 failed
