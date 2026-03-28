## Approach

The reverted PR #230 put `allowed_users` in `ZbobrDispatcherConfig` and changed the `TaskBackend::list_tasks` trait signature — a broad API change. This task takes a narrower approach: the filter belongs in the **GitHub task backend config** only.

## Key design decisions

- `allowed_usernames: Option<Vec<String>>` added to `ZbobrTaskBackendGithubConfig` (not dispatcher config).
- `TaskBackend::list_tasks` trait signature stays unchanged — no callers need updating.
- Filtering is done client-side after fetching the issues list, using the `user.login` field from GitHub's API response (already available; just needs to be deserialized).
- `None` or empty vec = no filter (all users accepted).
- The `IssueUser` struct and `user` field on `IssueResponse` are straightforward additions from the reverted PR — that part was correct; only the dispatcher-level placement was wrong.

## Files changed

1. `zbobr-task-backend-github/src/config.rs` — add `allowed_usernames` field
2. `zbobr-task-backend-github/src/github.rs` — add `IssueUser`/`user` deserialization + filter logic in `list_tasks` + test struct fixes
3. `zbobr/src/init.rs` — add `allowed_usernames: None` to default config template