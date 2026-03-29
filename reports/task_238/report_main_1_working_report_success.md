# Secret Type Implementation

## What was done

### 1. New `Secret` enum (`zbobr-api/src/secret.rs`)
- `Secret::Value(String)` — inline literal
- `Secret::Env(String)` — env var name resolved at runtime via `resolve() -> anyhow::Result<String>`
- Custom `serde::Deserialize` for TOML `{ value = "..." }` and `{ env = "VAR" }` forms
- Custom `serde::Serialize` for round-trip support
- `Default` impl returns `Secret::Value(String::new())`
- 9 unit tests covering deserialization, resolution, error cases, serialization

### 2. Token field migration (all 4 fields)
- `ZbobrDispatcherConfig::agent_github_token: String` → `Secret` with `#[config(skip_args)]`
- `ZbobrTaskBackendGithubConfig::github_token: String` → `Secret` with `#[config(skip_args)]` (removed `#[arg(long, env = "ZBOBR_TASK_GITHUB_TOKEN")]`)
- `ZbobrRepoBackendGithubConfig::github_token: String` → `Secret` with `#[config(skip_args)]` (removed `#[arg(long, env = "ZBOBR_REPO_GITHUB_TOKEN")]`)
- `ZbobrExecutorCopilotConfig::copilot_github_token: String` → `Secret` with `#[config(skip_args)]`
- Removed ad-hoc multi-env-var fallback chain from copilot executor `build()`; users now use `{ env = "VAR" }` directly

### 3. Callsite updates
- `zbobr-task-backend-github/src/github.rs`: `token.clone()` → `token.resolve()?`
- `zbobr-repo-backend-github/src/github.rs`: `token_auth_env()` changed to `anyhow::Result<[…; 3]>`, all 5 call sites now use `?`
- `zbobr-dispatcher/src/lib.rs`: `copilot_github_token()` returns `anyhow::Result<String>`
- `zbobr-dispatcher/src/cli.rs`: tokens resolved before `execute_tool` call; `execute_tool` signature updated to take `agent_token: &str` instead of `zbobr: &ZbobrDispatcher`
- `zbobr/src/init.rs`: Toml initializers updated to use `Secret::Value(...)`
- Integration test files updated: `Secret::Value(token)` in env.rs, `.resolve().expect(...)` in github_github.rs

### 4. Validation updated
- Both `validate()` methods now `resolve()` the secret and check for empty string with proper error messages

## Tests
All 53 zbobr-api tests pass, all 39 zbobr-dispatcher tests pass, 15 fs-fs integration tests pass, 0 failures.
