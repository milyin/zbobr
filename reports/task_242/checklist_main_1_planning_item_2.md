In `TaskBackendGithub::list_tasks` (in `github.rs`), add filtering logic after fetching the issues list and before constructing `GithubTaskWeak` entries:

- Read `self.inner.backend_config.allowed_usernames`.
- If it is `Some(users)` and `users` is non-empty: skip any issue whose `user.login` does not appear in the list (case-sensitive match, consistent with GitHub usernames).
- If it is `None` or an empty vec: no filtering, all issues are processed.

The `backend_config` is already stored on `ZbobrTaskBackendGithubImpl` so no new plumbing is needed. The `TaskBackend::list_tasks` trait signature stays unchanged — this is the key difference from the reverted approach.