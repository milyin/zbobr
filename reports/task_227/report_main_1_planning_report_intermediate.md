## Implementation Plan: allowed_users config

### Approach

Add an `allowed_users` config field to the dispatcher. Pass it to `list_tasks` as a new parameter. The GitHub backend filters by issue creator login; the FS backend ignores it.

### Changes

**1. `zbobr-api/src/config.rs` — add field to `ZbobrDispatcherConfig`**

Add `allowed_users: Vec<String>` annotated with `#[config(skip_args)]` (TOML-only, no CLI arg generated). Default is empty vec. In the `Default` impl for `ZbobrDispatcherConfig`, initialize to `Vec::new()`.

**2. `zbobr-api/src/backend.rs` — change `TaskBackend::list_tasks` signature**

Add `allowed_users: &[String]` parameter:
```
async fn list_tasks(&self, allowed_users: &[String]) -> anyhow::Result<Vec<Box<dyn TaskWeak>>>;
```

**3. `zbobr-task-backend-github/src/github.rs` — implement user filtering**

- Add `struct IssueUser { login: String }` (derives `serde::Deserialize`)
- Add `user: Option<IssueUser>` field to `IssueResponse`
- In `ArcTaskBackendGithub::list_tasks`: if `allowed_users` is non-empty, post-filter issues to those where `issue.user.as_ref().map(|u| u.login.as_str())` is in `allowed_users`
- Update signature

**4. `zbobr-task-backend-fs/src/fs.rs` — accept and ignore parameter**

Update both `ZbobrTaskBackendFs::list_tasks` and `ArcTaskBackendFs::list_tasks` signatures to accept `allowed_users: &[String]`. Ignore the parameter in both implementations.

**5. `zbobr-dispatcher/src/backend.rs` — update `DummyBackend`**

Update signature of `DummyBackend::list_tasks`. Ignore the parameter.

**6. `zbobr-dispatcher/src/task.rs` — update test helper `ArcTrackingBackend`**

Update `ArcTrackingBackend::list_tasks` signature. Ignore the parameter.

**7. `zbobr-dispatcher/src/cli.rs` — pass effective allowed_users to list_tasks**

In the manager loop, before calling `list_tasks`, compute:
```rust
let effective_allowed_users: Vec<String> = if zbobr.config().allowed_users.is_empty() {
    vec![zbobr.config().git_user_email.clone()]
} else {
    zbobr.config().allowed_users.clone()
};
let all_weak = match task_backend.list_tasks(&effective_allowed_users).await { ... }
```

**8. `zbobr/src/commands.rs` — pass empty slice to management list_tasks calls**

Both `task list` and `task show` (no id) call `list_tasks`. Pass `&[]` to show all tasks regardless of allowed_users (management commands should not filter).

### Key design decisions

- **Signature change on trait** (not config-based): cleanest match to "pass allowed users to the backend"
- **Post-filter in GitHub backend** (not API `creator` param): supports multiple users without multiple API calls; the GitHub `creator` param only accepts one value at a time
- **Empty = use git_user_email** computed at call site in manager loop (not at config build time): keeps the default logic in the dispatcher, not in config infrastructure
- **FS backend ignores**: as specified in task description
- **Management commands get `&[]`**: filtering is only relevant for the work loop, not for operators inspecting tasks
- **`#[config(skip_args)]`**: no CLI arg needed; the field is TOML-only, configured in zbobr.toml under `[dispatcher]`
