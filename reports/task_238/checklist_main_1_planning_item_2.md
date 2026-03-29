Every place that previously dereferenced a token field as `&str` or `.clone()` a `String` now needs to call `.resolve()?` to get the actual value.

**Validation methods** (`validate()`) — update emptiness checks:
- `if self.github_token.is_empty()` → call `self.github_token.resolve()` and check if it fails or returns empty; or check the `Secret` variants directly (e.g., `Secret::Value(s) if s.is_empty()` → error; `Secret::Env(s) if s.is_empty()` → error for missing var name)
- The validation error messages can stay the same; just update the condition

**Callsites in executors / backends** — find all uses of the token fields as string values (passed to octocrab `personal_token()`, set as env vars in child processes, etc.) and insert `.resolve()?` at each point. Key locations:
- `zbobr-repo-backend-github/src/github.rs` — `personal_token(backend_config.github_token.clone())`
- `zbobr-task-backend-github/src/github.rs` — same pattern
- `zbobr-dispatcher/src/cli.rs` — `copilot_github_token()` accessor and `agent_github_token` usage
- `zbobr-executor-claude/src/lib.rs` — env vars set for child process
- `zbobr-executor-copilot/src/lib.rs` — env vars set for child process

The `copilot_github_token()` accessor on `ZbobrDispatcher` (`zbobr-dispatcher/src/lib.rs`) currently returns `&str`; it needs to return `anyhow::Result<String>` or the callers must call `resolve()` themselves.

**`init.rs`** — the default config construction uses `Some("not-configured".into())` and `Some(String::new())` for token fields; replace these with `Some(Secret::Value("not-configured".to_string()))` and `Some(Secret::Value(String::new()))` accordingly.