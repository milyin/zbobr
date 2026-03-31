# Secret Struct Implementation — Complete

## What was done

### 1. New `Secret` struct in `zbobr-utility/src/secret.rs`
- Private `SecretSource` enum (Value/Env) — public enum form removed
- Private `resolved: Option<String>` field
- `Secret::value(s)` and `Secret::env(var)` constructors
- `resolve(&mut self) -> anyhow::Result<&str>` — fetches and caches the value
- `is_resolved(&self) -> bool`
- `impl AsRef<str>` — panics if `resolve()` was not called first
- `impl Clone` — derived, preserves resolved state
- Custom `Serialize`/`Deserialize` — same TOML format `{ value = "..." }` / `{ env = "..." }`
- 13 tests including panic tests, clone preservation, env var tests

### 2. `zbobr-utility/Cargo.toml`
- Added `serde` dependency and `toml` dev-dependency

### 3. `zbobr-utility/src/lib.rs`
- Added `pub mod secret; pub use secret::Secret;`

### 4. `zbobr-api/src/secret.rs`
- Replaced with single re-export: `pub use zbobr_utility::Secret;`

### 5. `zbobr-api/src/config.rs`
- Import changed to `use zbobr_utility::Secret`
- `Secret::Value("not-configured")` → `Secret::value("not-configured")`
- `validate(&self)` → `validate(&mut self)`, now calls `self.agent_github_token.resolve()`

### 6. Config validate() methods — `&self` → `&mut self`
- `ZbobrTaskBackendGithubConfig::validate`
- `ZbobrRepoBackendGithubConfig::validate`

### 7. `from_config` functions — pre-resolved via validate
- `ZbobrTaskBackendGithubImpl::from_config` — `mut backend_config`, uses `as_ref()`
- `ZbobrRepoBackendGithub::from_config` — `mut backend_config`, uses `as_ref()`
- `ZbobrRepoBackendGithub::token_auth_env` — uses `as_ref()` (token already resolved)

### 8. `ZbobrDispatcher`
- `validated(mut self)` now also resolves `copilot_github_token`
- `copilot_github_token(&self) -> &str` (was `-> anyhow::Result<String>`)

### 9. `zbobr-dispatcher/src/cli.rs`
- `copilot_token_owned`: `copilot_github_token().to_owned()` (no more `?`)
- `agent_token_owned`: `config().agent_github_token.as_ref().to_owned()`

### 10. Integration tests
- `env.rs`: pre-resolves `dispatcher_config.agent_github_token` before building `Arc<ZbobrDispatcher>` (since `validated()` not called in tests due to McpTester incompatibility with model check); `Secret::value(...)` constructors used
- `integration_github_github.rs`: resolve() on temporary Secret fixed with local `mut` bindings

## Test results
- All 13 new Secret tests in zbobr-utility pass
- Only pre-existing failure: `context::tests::compact_comment_uses_first_line_only` (unrelated to this task)
- Build succeeds cleanly
