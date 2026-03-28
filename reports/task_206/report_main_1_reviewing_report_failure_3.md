## Review summary (task 206)
The change set is small and directly related to the task. It correctly:
- Prefixes task error with a ❌ mark and a timestamp formatted via the existing context timestamp formatter.
- Clears `task.error` when transitioning from non-running → running in `TaskSession::set_state`.
- Updates the unit test to tolerate formatted error and assert timestamp presence.

However, there are two important issues that should be fixed.

---

## 1) Invariant not enforced at the generic state setter (potential bypass)
### Finding
`zbobr-api/src/backend.rs` default `TaskMut::set_state` still only does:
```rust
task.state = state;
```
It does **not** clear `task.error` on entering Running.

`TaskBackendExt::set_task_state()` calls `mutable.set_state(state)` (the generic path), so any caller that sets a task to a running state via this path will **not** clear error, violating the requirement (“when task enters to running state, clear the error”). Today the dispatcher uses `TaskSession::set_state`, but this is a footgun and creates future inconsistencies.

### Recommendation (should-fix)
Enforce the invariant in the lowest-level canonical setter as well:
- Update `TaskMut::set_state` to clear `task.error` only on non-running → running transition, mirroring the dispatcher logic.
- Optionally keep the dispatcher-side clearing (harmless) or remove it once the invariant is centralized.

This strengthens correctness and makes behavior robust against partial/inconsistent call-path changes.

---

## 2) Duplicated ❌/timestamp formatting logic + repeated literal
### Finding
Error formatting is duplicated in:
- `zbobr-api/src/backend.rs` (`TaskMut::set_error`)
- `zbobr-dispatcher/src/task.rs` (`RoleSession::set_error`)

Both embed the same `"\u{274C}"` literal and build the same string format. The project rules explicitly ask to avoid repeated string literals to prevent divergence.

Also, note there is already a ❌ literal in the context markdown prefix code (`zbobr-api/src/context/mod.rs`), so the symbol is now represented in multiple ways/places.

### Recommendation (should-fix)
Introduce a single canonical representation, e.g. in `zbobr-api`:
- `pub const ERROR_MARK: char = '❌';` (and/or `pub const ERROR_MARK_STR: &str = "❌";`)
- A helper like `pub fn format_error_with_timestamp(ts: &DateTime<FixedOffset>, msg: &str) -> String` that uses `format_timestamp`.

Then reuse from both call sites. This reduces drift risk and improves refactor safety.

---

## Analog/pattern consistency
✅ Good analog choice: `format_timestamp` is taken directly from the existing stage title/context timestamp formatting, which matches the requirement and keeps formatting consistent.

⚠️ Minor consistency note: `RoleSession::set_error` uses dispatcher-config fixed offset; `TaskMut::set_error` uses UTC fixed offset (+0000). This may be acceptable, but consider aligning if a non-UTC display is desired everywhere.

---

## Extraneous changes
None noted; all edits are tied to error formatting/timestamping or running-state clearing.

---

## Overall assessment
Functionally close, but I recommend fixing (1) and (2) before merge to avoid behavior bypasses and to comply with the project’s “no repeated literals” rule.