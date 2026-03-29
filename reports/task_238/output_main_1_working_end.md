All done. Here's a summary of what was implemented:

## Summary

**New `Secret` enum** in `zbobr-api/src/secret.rs`:
- `Secret::Value(String)` — inline literal
- `Secret::Env(String)` — env variable resolved at runtime
- `resolve() -> anyhow::Result<String>` method
- TOML deserialization: `{ value = "tok" }` or `{ env = "MY_VAR" }`
- 9 unit tests included

**4 token fields migrated** from `String` to `Secret`:
- `ZbobrDispatcherConfig::agent_github_token`
- `ZbobrTaskBackendGithubConfig::github_token` (removed `#[arg]` CLI flag)
- `ZbobrRepoBackendGithubConfig::github_token` (removed `#[arg]` CLI flag)  
- `ZbobrExecutorCopilotConfig::copilot_github_token` (removed ad-hoc env fallback chain)

All fields are TOML-only (`#[config(skip_args)]`) since a table value can't be expressed as a CLI arg.

**Callsites updated**: `token_auth_env()` now returns `anyhow::Result`, `execute_tool` takes explicit `agent_token: &str`, validation uses `resolve()?`, integration tests use `Secret::Value(...)`.