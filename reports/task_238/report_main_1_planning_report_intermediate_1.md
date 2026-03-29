# Plan: Secret type redesign (resolve + AsRef contract)

## Approach

Redesign `Secret` from a public enum to a struct with private fields that enforces the "resolve before access" contract.

### New `Secret` structure

```
struct Secret {
    source: SecretSource,   // private enum: Value(String) | Env(String)
    resolved: Option<String>,  // cached resolved value; None = not yet resolved
}
```

- `Secret::value(s)` and `Secret::env(var)` constructors replace public enum variants
- `resolve(&mut self) -> Result<&str, anyhow::Error>` — resolves from source, stores in `resolved`, returns `&str`. Idempotent: re-calling returns the cached value.
- `impl AsRef<str>` — panics with a clear message if `resolved` is `None`; otherwise returns `&str`
- `Clone` — copies source, resets `resolved` to `None` (forces re-resolution after clone)
- Serde serialize/deserialize — touches source only, ignores cache (same TOML format)
- `Default` — empty `Value("")` source, unresolved

### Impact cascade

1. **`validate()` → `validate(&mut self)`** in 3 configs:
   - `ZbobrDispatcherConfig::validate()` — add `agent_github_token.resolve()?` (currently only checks tool/model)
   - `ZbobrTaskBackendGithubConfig::validate()` — already calls resolve, keep it (just needs `&mut self`)
   - `ZbobrRepoBackendGithubConfig::validate()` — same

2. **`from_config()` constructors** in github backends take value, so add `mut` keyword to `backend_config` — validate now takes `&mut self` so `backend_config.validate()?` works.

3. **Usage sites switch from `resolve()` to `as_ref()`** (pre-resolved during validate/from_config):
   - `zbobr-repo-backend-github/src/github.rs` lines 152 and 270 → `as_ref()`
   - `zbobr-task-backend-github/src/github.rs` line 190 → `as_ref()`
   - `zbobr-dispatcher/src/cli.rs:482` → `as_ref()` (agent token pre-resolved in `ZbobrDispatcherConfig::validate()`)

4. **`ZbobrDispatcher::validated(mut self)`** — add resolution of copilot token before returning:
   `self.copilot.config.copilot_github_token.resolve()?;`
   Then change `copilot_github_token(&self) -> anyhow::Result<String>` → `copilot_github_token(&self) -> &str` using `as_ref()`.

5. **Constructors in `init.rs` and integration test `env.rs`** — replace `Secret::Value(...)` with `Secret::value(...)`.

6. **Integration test `integration_github_github.rs`** — the `.resolve()` calls at setup time that return `String` (lines 36, 39) must become `mut` bindings with `.resolve().expect("...").to_string()`.

7. **Update `Secret` tests** in `secret.rs` — remove pattern matching on private enum variants; test via `resolve()` + `as_ref()` instead.

### Design rationale

- `resolve()` is the single point of truth for fetching from env — called once at validation, cached for all subsequent accesses
- `as_ref()` provides zero-cost access after resolution; panics are intentional programmer errors (not runtime errors), consistent with Rust idioms like `unwrap()`
- `Clone` resets resolved state so that cloned config objects (e.g., when building executor in `build_executor()`) remain safe and don't silently share stale resolved state from a different context
