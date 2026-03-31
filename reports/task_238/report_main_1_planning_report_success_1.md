## Plan summary

### Approach
Refactor `Secret` from a public enum in `zbobr-api` to a struct with private fields in `zbobr-utility`, implementing the resolve-then-access contract the user specified.

### Key design decisions

**Struct with private fields instead of public enum:**
- `SecretSource` enum is private (not exported)
- `Secret` struct has private `source: SecretSource` and `resolved: Option<String>`
- Public API: `Secret::value()`, `Secret::env()`, `resolve(&mut self)`, `is_resolved()`, `AsRef<str>` (panics if unresolved)

**Clone preserves resolved state:**
- `#[derive(Clone)]` naturally copies both `source` and `resolved`, so a clone of a resolved secret is also resolved — no special handling needed.

**Resolve is the gate:**
- `resolve(&mut self)` caches the value. Idempotent.
- `AsRef<str>` panics if called before successful resolve — "early panic is better" per user.
- No `is_resolved()` checks in work code — just call `as_ref()` after validate.

**Serde: resolved cache is not serialized:**
- Deserialized secrets always start unresolved.
- Same TOML format: `{ value = "..." }` or `{ env = "..." }`.

**Validate methods become `&mut self`:**
- `ZbobrRepoBackendGithubConfig::validate()` and `ZbobrTaskBackendGithubConfig::validate()` — the only two configs whose validate methods call resolve on secrets.

### Analog
The custom Deserialize pattern follows the same structure as `StageTransition` in `zbobr-api/src/config.rs` (untagged helper enum for flexible TOML deserialization).

### Checklist items
1. Implement `Secret` struct in `zbobr-utility` (with tests)
2. Remove `secret.rs` from `zbobr-api`, re-export from `zbobr-utility`
3. Update `validate()` signatures, construction callsites (`Secret::Value` → `Secret::value`), and access callsites (`.resolve()?` → `.as_ref()`)
