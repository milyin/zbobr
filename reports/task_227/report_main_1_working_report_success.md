# Implementation Summary

## Changes Made

### 1. `zbobr-api/src/config.rs`
- Added `allowed_users: Option<Vec<String>>` field to `ZbobrDispatcherConfig` (with `#[arg(long)]` for CLI support and TOML deserialization via the `config_struct` macro)
- Added `effective_allowed_users() -> Vec<String>` method: returns the configured list if `Some`, otherwise falls back to `[git_user_email]` (or empty vec if email is also unset)
- Updated `Default` impl to include `allowed_users: None`

### 2. `zbobr-api/src/backend.rs`
- Updated `TaskBackend::list_tasks` trait signature to `async fn list_tasks(&self, allowed_users: &[String]) -> anyhow::Result<Vec<Box<dyn TaskWeak>>>`
- Empty slice means no filtering

### 3. `zbobr-task-backend-github/src/github.rs`
- Added `IssueUser { login: String }` struct (serde Deserialize)
- Added `user: Option<IssueUser>` field to `IssueResponse` with `#[serde(default)]`
- Updated `list_tasks` to filter issues: if `allowed_users` is non-empty, only include issues whose `user.login` matches an entry in the list
- Updated test helper `make_issue_with_params` to include `user: None`

### 4. `zbobr-task-backend-fs/src/fs.rs`
- Updated both `list_tasks` implementations (`ZbobrTaskBackendFs` and `ArcTaskBackendFs`) to accept `_allowed_users: &[String]` (FS backend ignores it per spec)

### 5. `zbobr-dispatcher/src/backend.rs`
- Updated `DummyBackend::list_tasks` signature to match new trait

### 6. `zbobr-dispatcher/src/task.rs`
- Updated `ArcTrackingBackend::list_tasks` (test mock) signature to match new trait

### 7. `zbobr-dispatcher/src/cli.rs`
- In `run_manager_loop`, compute `zbobr.config().effective_allowed_users()` and pass to `task_backend.list_tasks(&allowed_users)`

### 8. `zbobr/src/commands.rs`
- CLI task list/show commands pass `&[]` to `list_tasks` (no filtering — show all tasks)

### 9. `zbobr/src/init.rs`
- Added `allowed_users: None` to the default config template

## Behavior
- If `allowed_users` is not configured in `zbobr.toml`, the dispatcher defaults to only processing tasks created by the user identified by `git_user_email`
- If `allowed_users` is configured, only tasks from those GitHub usernames are processed
- The FS backend ignores `allowed_users` (as specified)
- CLI commands (`zbobr task list`, `zbobr task show`) are not affected by filtering
