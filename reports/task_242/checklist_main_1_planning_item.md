In `zbobr-task-backend-github/src/config.rs`, add an optional `Vec<String>` field `allowed_usernames` to `ZbobrTaskBackendGithubConfig`.

- Add `#[arg(long)]` annotation (consistent with other optional array fields in configs).
- Type should be `Option<Vec<String>>` — `None` means no filtering (all users allowed).
- Add a doc comment explaining the field: when set, only tasks created by these GitHub usernames will be processed.
- No validation needed — an empty vec is fine (treated as no filter, same as `None`).

This is the only config change needed. No changes to `ZbobrDispatcherConfig` or the `TaskBackend` trait.