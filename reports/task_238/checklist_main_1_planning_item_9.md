## Goal
Propagate the new `Secret` API throughout the codebase: validation resolves secrets, callsites use `as_ref()` for access.

## Changes

### 1. `validate()` signatures: `&self` → `&mut self`

These configs have `validate()` methods that call `resolve()` on Secret fields. Since `resolve()` is now `&mut self`, validate must also be `&mut self`:
- `ZbobrRepoBackendGithubConfig::validate(&mut self)` in `zbobr-repo-backend-github/src/config.rs`
- `ZbobrTaskBackendGithubConfig::validate(&mut self)` in `zbobr-task-backend-github/src/config.rs`

In those validate methods, the pattern changes from:
```
let token = self.github_token.resolve()?;
if token.is_empty() { bail!(...) }
```
to:
```
self.github_token.resolve()?;
if self.github_token.as_ref().is_empty() { bail!(...) }
```

### 2. Non-validate callsites: `.resolve()?` → `.as_ref()`

These callsites currently call `.resolve()` at use time. After validate() resolves and caches the value, they should just use `.as_ref()` (which panics if unresolved — acceptable per the "early panic is better" design):

- `zbobr-repo-backend-github/src/github.rs`: `backend_config.github_token.resolve()?` (2 occurrences) → `backend_config.github_token.as_ref()`
- `zbobr-task-backend-github/src/github.rs`: `backend_config.github_token.resolve()?` → `backend_config.github_token.as_ref()`
- `zbobr-dispatcher/src/cli.rs:482`: `self.zbobr.config().agent_github_token.resolve()?` → `self.zbobr.config().agent_github_token.as_ref()`
- `zbobr-dispatcher/src/lib.rs:127`: `copilot_github_token()` currently returns `Result<String>` from resolve(). Change to return `String` (or keep `Result<String>` by wrapping as_ref). Since the user said "early panic is better", simplest is to return `String::from(self.copilot.config.copilot_github_token.as_ref())` with return type `String`. Update the caller in `cli.rs` accordingly (remove the `?` on the call).

### 3. Construction callsites: `Secret::Value(...)` → `Secret::value(...)`

All direct enum variant constructors must be replaced with the new public constructors:

- `zbobr-api/src/config.rs` (Default impl): already handled in checklist item 2
- `zbobr/src/init.rs`: 3 occurrences of `Secret::Value(...)` → `Secret::value(...)`
- `zbobr-dispatcher/tests/mcp_integration/env.rs`: 2 occurrences of `Secret::Value(...)` → `Secret::value(...)`

### 4. Integration tests

- `zbobr-dispatcher/tests/integration_github_github.rs:35-39`: calls `.resolve()` on secrets extracted from config. Since validate() would have resolved them, change to `.as_ref()`. But these tests may also need the secrets to be resolved explicitly if validate() wasn't called on that config. Check the test context and either call resolve() directly or use as_ref() after ensuring resolution.

### 5. Commented-out tests in `zbobr-dispatcher/src/config.rs`

Lines 103-165 are inside a `/* ... */` block and compare `config.agent_github_token` via `PartialEq<str>`. Since `Secret` no longer implements `PartialEq<str>`, those assertions would fail if uncommented. Update those assertions to use `config.agent_github_token.as_ref()` after calling resolve (or just leave them commented if they remain in the block).

### 6. Run tests
After all changes, run `cargo test` to confirm all tests pass.
