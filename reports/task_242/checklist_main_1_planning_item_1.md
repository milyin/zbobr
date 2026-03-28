In `zbobr-task-backend-github/src/github.rs`, extend `IssueResponse` to capture the issue author:

- Add a new `IssueUser` struct with a single `login: String` field (deserialize from GitHub API's `user.login`).
- Add `#[serde(default)] user: Option<IssueUser>` to `IssueResponse`.

This mirrors exactly what the reverted PR #230 did in the GitHub backend (that part of the revert was a collateral loss, not the reason for revert).

Also update test helper `make_issue_with_params` (and any other `IssueResponse` struct literals in tests) to include `user: None` so they still compile.