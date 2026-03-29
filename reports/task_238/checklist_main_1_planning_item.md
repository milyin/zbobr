Add a `Secret` enum to `zbobr-api` (e.g., as a module `zbobr-api/src/secret.rs` re-exported from `lib.rs`).

The enum has two variants:
- `Value(String)` — the secret is stored inline as a literal string
- `Env(String)` — the secret is resolved at runtime from the named environment variable

**Serde implementation** — use an untagged helper with distinct fields:
```
{ value = "secret-string" }  → Secret::Value(...)
{ env = "ENV_VAR_NAME" }     → Secret::Env(...)
```
Implement `serde::Deserialize` and `serde::Serialize` manually or via an untagged helper struct (the same approach used by `StageTransition` in `zbobr-api/src/config.rs` — that type also deserializes from either a plain string or a full table).

**`resolve()` method** — returns `anyhow::Result<String>`:
- `Value(s)` → `Ok(s.clone())`
- `Env(name)` → `std::env::var(name)` with a descriptive error if the variable is unset

**`Default` impl** — `Secret::Value(String::new())` (matches the current string default for token fields).

No backward-compatible string shorthand needed (task says old plain-string format is not allowed).