The plan is ready for review. Here's the summary:

**Approach:** Add `allowed_users: Vec<String>` to `ZbobrDispatcherConfig`, change the `TaskBackend::list_tasks` trait to accept `allowed_users: &[String]`, and update all implementations.

**Key changes:**
1. `zbobr-api/src/config.rs` — add `allowed_users: Vec<String>` with `#[config(skip_args)]` (TOML-only, default `[]`)
2. `zbobr-api/src/backend.rs` — add `allowed_users: &[String]` param to `list_tasks` trait method
3. `zbobr-task-backend-github` — add `user.login` to `IssueResponse`, post-filter by login when `allowed_users` is non-empty
4. `zbobr-task-backend-fs` — accept and ignore the new param
5. `zbobr-dispatcher/src/backend.rs` + `task.rs` — update `DummyBackend` and test helper signatures
6. `zbobr-dispatcher/src/cli.rs` — compute effective allowed_users (defaults to `[git_user_email]` when empty), pass to `list_tasks`
7. `zbobr/src/commands.rs` — pass `&[]` to management commands (`task list`, `task show`)

**Design decisions:**
- Trait signature change (not config injection) to match "pass to backend" intent
- Post-filter in GitHub backend (avoids multiple API calls for multi-user case)
- Empty = `git_user_email` computed in manager loop, not at config-build time
- FS backend ignores as specified