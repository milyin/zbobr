## What

Change the `list_tasks` method signature in the `TaskBackend` trait in `zbobr-api/src/backend.rs` from:

```
async fn list_tasks(&self) -> anyhow::Result<Vec<Box<dyn TaskWeak>>>;
```

to accept a `allowed_users: &[String]` parameter. This parameter represents the list of GitHub user logins (or email addresses) whose tasks should be included. An empty slice means "no filter — include all".

## All implementations to update

There are multiple `TaskBackend` implementations that must be updated:
1. `zbobr-task-backend-github/src/github.rs` — main GitHub backend
2. `zbobr-task-backend-fs/src/fs.rs` — filesystem backend (two impl blocks, lines ~529 and ~642)
3. `zbobr-dispatcher/src/backend.rs` — dispatcher's local backend wrapper
4. `zbobr-dispatcher/src/task.rs` — dispatcher's in-memory backend

For items 2–4, just add the parameter and ignore it (pass-through if wrapping another backend, or ignore entirely if not applicable).

## Why

The task design calls for passing filtering intent to the backend rather than filtering after the fact. This way, the GitHub backend can apply the filter efficiently, while other backends can opt out. An empty slice as "no filter" preserves backward compatibility for call sites that don't need filtering (e.g., admin CLI commands).