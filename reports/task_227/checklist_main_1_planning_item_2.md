## What

In `zbobr-task-backend-github/src/github.rs`, update the GitHub backend's `list_tasks` to filter issues by creator:

1. Add a `user` field to the `IssueResponse` struct to deserialize the issue creator from the GitHub API response. The GitHub API returns `"user": {"login": "..."}` for each issue. Add a small `IssueUser { login: String }` struct and a `user: IssueUser` field to `IssueResponse`.

2. In `list_tasks`, after fetching all issues, filter the list to only include issues where `issue.user.login` is in the `allowed_users` slice. If `allowed_users` is empty, include all issues (no-op filter).

## Why

The GitHub Issues API does have a `creator` query parameter, but it only supports one creator at a time. Filtering in-memory after a single API call is simpler and handles the multi-user case correctly.

## Pattern to follow

The existing filtering in `list_tasks` already skips malformed issues with `tracing::warn`. Follow the same log-and-skip style for filtered-out issues if logging is desired, but a silent filter is also acceptable.