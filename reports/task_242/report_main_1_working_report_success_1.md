# Fix: allowed_usernames filter using GitHub API creator param

## Changes

### `zbobr-dispatcher/tests/mcp_integration/env.rs`
- Added missing `allowed_usernames: None` field to `ZbobrTaskBackendGithubConfig` initializer in test env setup. This was the compilation failure reported by the testing stage.

### `zbobr-task-backend-github/src/github.rs`
- Removed `IssueUser` struct and `user` field from `IssueResponse` (no longer needed)
- Refactored `list_tasks` to use GitHub API `creator` query param when `allowed_usernames` is set: makes one API call per allowed username and merges results, rather than fetching all issues and filtering client-side
- Removed `user: None` from test helper `make_issue_with_params`

## Result
All tests pass. Server-side filtering reduces unnecessary data transfer when `allowed_usernames` is configured.