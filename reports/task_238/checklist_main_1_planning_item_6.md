All `validate` methods that call `resolve()` must change from `&self` to `&mut self` since `resolve` now requires mutable access. Additionally, all construction callsites using variant syntax must switch to constructor methods.

**validate() signature changes:**
- `ZbobrRepoBackendGithubConfig::validate(&self)` → `validate(&mut self)` in `zbobr-repo-backend-github/src/config.rs`. The existing logic calls `self.github_token.resolve()` and checks for empty; adapt to the new signature (`.resolve()` returns `&str`, check `is_empty()` on that).
- `ZbobrTaskBackendGithubConfig::validate(&self)` → `validate(&mut self)` in `zbobr-task-backend-github/src/config.rs`. Same adaptation.
- Find any other callers of `validate()` that pass `&self` references and verify they have mutable access to the config; update call sites as needed (e.g., `backend_config.validate()` calls may need `&mut backend_config`).

**Construction callsites to update** (all files that create `Secret::Value(...)` or `Secret::Env(...)`) — these are currently located in:
- `zbobr-api/src/config.rs` (two occurrences)
- `zbobr/src/init.rs` (three occurrences)
- `zbobr-dispatcher/tests/mcp_integration/env.rs` (two occurrences)
Change all to use `Secret::value(...)` or `Secret::env(...)` constructor methods.