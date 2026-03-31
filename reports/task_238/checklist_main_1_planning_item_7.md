## Goal
Move `Secret` from `zbobr-api` to `zbobr-utility` and refactor it from a public enum to a struct with private fields that enforces explicit resolution before access.

## What to build

Create `zbobr-utility/src/secret.rs` with:

**Private source enum** (not exported):
- `SecretSource::Value(String)` — inline literal
- `SecretSource::Env(String)` — environment variable name

**Public `Secret` struct** with private fields:
- `source: SecretSource`
- `resolved: Option<String>` — `None` means unresolved; `Some(s)` means already resolved (cached)

**Methods:**
- `Secret::value(s: impl Into<String>) -> Self` — constructor for inline literal
- `Secret::env(var: impl Into<String>) -> Self` — constructor for env var
- `fn resolve(&mut self) -> anyhow::Result<&str>` — resolves and caches the value. For `Value`, copies the string. For `Env`, reads the environment variable and returns an error if not set. Idempotent: if already resolved, returns cached value immediately without re-reading env.
- `fn is_resolved(&self) -> bool` — returns whether resolve() has been called and succeeded
- `impl AsRef<str>` — returns the resolved value. **Panics** if `resolve()` was never called or failed. Error message should be clear: "Secret must be resolved before access — call resolve() first"
- `impl Default` — returns `Secret::value("")`
- `impl Clone` — use `#[derive(Clone)]` since both fields implement Clone. This naturally preserves the resolved state in the clone, which is the desired behavior.
- `impl serde::Serialize` / `impl serde::Deserialize` — same TOML format as before: `{ value = "..." }` or `{ env = "..." }`. The resolved cache is NOT serialized; it's always deserialized as unresolved.
- `impl Debug` — do NOT expose the resolved value in debug output to avoid leaking secrets. Show source only, e.g., `Secret::Value` or `Secret::Env("VAR_NAME")`.

**Important constraint:** `resolved` field must not be included in Serde. A deserialized Secret is always unresolved (resolved = None).

**Tests to add:**
- `resolve()` works for Value variant
- `resolve()` works for Env variant (set env var in test)
- `resolve()` returns error for missing env var
- `resolve()` is idempotent (calling twice returns same result)
- `as_ref()` panics if never resolved
- `as_ref()` returns value after resolve
- Clone preserves resolved state (clone of resolved secret is also resolved)
- `is_resolved()` returns false before resolve, true after
- Serde roundtrip for both Value and Env forms (deserialized secret is unresolved)

## Dependencies
Add `serde` to `zbobr-utility/Cargo.toml` as a workspace dependency.

## Export
In `zbobr-utility/src/lib.rs`, add `pub mod secret;` and `pub use secret::Secret;`.
