Plan finalized and checklist submitted for implementation. The 4 items cover:

1. **Define `Secret` enum** in `zbobr-api` — `Value(String)` / `Env(String)` with custom serde and `resolve()` method
2. **Migrate 4 token fields** — `agent_github_token`, two `github_token`s, `copilot_github_token` — removing clap env attrs and the ad-hoc env var fallback in the copilot build
3. **Update callsites** — validation, octocrab init, child process env vars, `init.rs` defaults
4. **Tests** — unit tests for `Secret` serde/resolve + update existing token tests