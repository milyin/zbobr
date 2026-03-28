Plan approved and checklist created. Here's the summary of what will be implemented:

1. **Add `allowed_users: Vec<String>` to `ZbobrDispatcherConfig`** — optional in TOML; if not set, defaults to `[git_user_email]` after config build.

2. **Update `TaskBackend::list_tasks` signature** — add `allowed_users: &[String]` parameter (empty = no filter).

3. **GitHub backend filtering** — add `user: IssueUser` to `IssueResponse` to capture the issue creator login, then filter the issue list in-memory against `allowed_users`.

4. **FS backend** — accept the new parameter, ignore it.

5. **Update all call sites** — dispatcher dispatch loop passes `config.allowed_users`; admin CLI commands pass `&[]` (show all).