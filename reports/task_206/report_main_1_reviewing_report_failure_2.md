# Review: task-206 make error up-to-date

## Scope / diff inspected
Compared `origin/main...HEAD`. Only two files changed:
- `zbobr-dispatcher/src/task.rs`
- `zbobr-api/src/backend.rs`

No extraneous changes beyond task requirements.

## Requirements coverage
### 1) Add red X + timestamp when setting error
- Implemented in:
  - `RoleSession::set_error` (dispatcher): prefixes `\u{274C}` + formatted timestamp + message.
  - `TaskMut::set_error` (api backend trait default): same prefixing.
- Symbol choice `U+274C` matches “red X” requirement.

**Issue: timestamp formatting is duplicated rather than reused from the existing “context” timestamp code.**
There is already analogous timestamp formatting code in `zbobr-api/src/context/stage_title.rs`:
```rust
fn format_timestamp(ts: &DateTime<FixedOffset>) -> String {
    format!("{} {}", ts.format("%Y-%m-%d %H:%M:%S"), ts.format("%z"))
}
```
New code re-implements this exact formatting in 2 places. This violates the project rule “avoid repeated string literals” and increases drift risk.

### 2) Clear error when task enters running
- Implemented in `TaskSession::set_state` (dispatcher):
```rust
if task.state != state && state.is_running() {
    task.error = None;
}
```

**Correctness issue: this clears on `Running(...) -> Running(...)` transitions too.**
Because `State::Running(pipeline, stage)` changes as stages change, `task.state != state` can be true while already running. Requirement (and prior review feedback in this task) is to clear only when *entering* Running from a non-running state.

Expected logic:
```rust
if !task.state.is_running() && state.is_running() {
    task.error = None;
}
```
This matches “enters running state” precisely and avoids accidental clears during running-stage-to-running-stage transitions.

## Consistency with analog / existing patterns
- Dispatcher timestamps elsewhere (e.g. `zbobr-dispatcher/src/cli.rs:422`) use:
  - `chrono::Utc::now().with_timezone(&self.zbobr.config().fixed_offset())`
  This matches what `RoleSession::set_error` does. Good.

- Structured comment timestamps in FS backend use local fixed offset:
  - `Utc::now().with_timezone(Local::now().offset())`

**Potential inconsistency:** `TaskMut::set_error` uses bare `Utc::now()` (so offset is always `+0000`). That may be acceptable, but it’s not aligned with the dispatcher’s configured offset / the “context” style (FixedOffset). If consistent display is desired, prefer a FixedOffset (local or configured) here too.

## Robustness / compile-time / drift resistance
### Duplicated literals
The following are duplicated and should be centralized:
- The ❌ symbol (`"\u{274C}"` in two places, plus a char check in test)
- Timestamp format strings (`"%Y-%m-%d %H:%M:%S"` and `"%z"`) used in multiple modules

Recommendation:
- Introduce a single canonical constant, e.g. `const ERROR_MARK: char = '❌';` or `const ERROR_MARK_STR: &str = "❌";`
- Expose a shared timestamp formatter (or reuse an existing one by making it accessible) so new call sites don’t reimplement formatting.

### Double-formatting possibility
Both `RoleSession::set_error` and `TaskMut::set_error` now apply formatting. If any code path ever calls both (or passes already-formatted strings), you could end up with double ❌+timestamp prefixes. Not necessarily occurring today, but worth guarding against by centralizing formatting at the lowest common layer or by detecting an existing prefix.

## Tests
`comment_model_tests` was updated to avoid exact-string matching (good), but it currently only asserts:
- starts with ❌
- contains "oops"

**Missing assertion:** timestamp presence/shape. Since timestamp is a core requirement, consider validating it without being too brittle, e.g.:
- regex match `^❌ \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} [+-]\d{4} `
- or parse the timestamp substring with `DateTime::parse_from_str` using the same format.

## Overall assessment
- Implementation is close, but I’m marking **FAIL** due to the state-transition logic not strictly matching “entering running” semantics (clears on Running→Running transitions) and due to duplicated formatting logic that conflicts with repo guidance on avoiding repeated literals.

## Required fixes
1) Change error-clearing condition in `TaskSession::set_state` to only clear when transitioning from non-running to running.
2) Centralize ❌ symbol and timestamp formatting to avoid drift (at minimum, a const for the symbol and a helper for timestamp string).
3) Strengthen the updated test to assert timestamp presence/format in addition to ❌ + message.
