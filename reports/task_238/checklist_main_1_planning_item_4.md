Create `zbobr-utility/src/secret.rs` with the refactored `Secret` type and expose it via `zbobr-utility/src/lib.rs`.

**What to build:**
- A `Secret` struct with two private fields: `source: SecretSource` (a private enum with `Value(String)` and `Env(String)` variants) and `resolved: Option<String>` (the cached resolved value).
- Public constructor methods: `Secret::value(s: impl Into<String>) -> Self` and `Secret::env(var: impl Into<String>) -> Self`. Both produce an unresolved secret.
- `resolve(&mut self) -> anyhow::Result<&str>`: if already resolved, returns the cached ref; otherwise fetches from source (reads env var for `Env`), stores in `resolved`, and returns `&str` to the cached value. Returns an error if resolution fails (e.g. missing env var).
- `is_resolved(&self) -> bool`: returns whether `resolve` has been successfully called.
- `impl AsRef<str> for Secret`: returns `self.resolved.as_deref().expect("Secret::resolve() must be called before accessing the value")`.
- `impl Default for Secret`: returns `Secret::value("")` in unresolved state.
- Derive `Clone` — this naturally copies both `source` and `resolved`, so a cloned resolved secret remains resolved.
- Custom `Serialize`/`Deserialize` implementations matching the existing TOML format: `{ value = "..." }` or `{ env = "..." }`. Deserialization always produces an unresolved secret regardless of variant.
- Unit tests covering: deserialize both forms, serialize both forms, resolve Value, resolve Env (set and missing), AsRef after resolve, AsRef panic before resolve, is_resolved, clone preserves resolved state.

**Dependencies needed:**
- Add `serde = { workspace = true }` to `zbobr-utility/Cargo.toml`.
- Add `anyhow = { workspace = true }` is already present in zbobr-utility; verify it's there.
- Add `toml = { workspace = true }` to dev-dependencies for tests.
- Add `mod secret; pub use secret::Secret;` to `zbobr-utility/src/lib.rs`.