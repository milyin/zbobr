Plan is finalized and sent to implementation. Here's the summary:

**3 checklist items created:**

1. **Implement `Secret` struct in `zbobr-utility`** — private `SecretSource` enum, struct with `source` + `resolved: Option<String>`, `resolve(&mut self)` caches value, `AsRef<str>` panics if unresolved, `Clone` via derive (naturally preserves resolved state), `is_resolved()`, same Serde TOML format, Debug that doesn't leak the value.

2. **Update `zbobr-api`** — delete `secret.rs`, change `pub use secret::Secret` to `pub use zbobr_utility::Secret`, update Default impl to use `Secret::value("not-configured")`.

3. **Update callsites** — `validate(&self)` → `validate(&mut self)` in the two GitHub backend configs; non-validate callsites in `github.rs`/`cli.rs` switch from `.resolve()?` to `.as_ref()`; all `Secret::Value(...)` constructors in `init.rs` and test files → `Secret::value(...)`.