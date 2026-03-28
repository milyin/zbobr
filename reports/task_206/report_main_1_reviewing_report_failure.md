# Review: task-206 make error up-to-date

## Scope / diff inspected
Compared `origin/main...HEAD`.
Only functional changes are in `zbobr-dispatcher/src/task.rs`:
- `RoleSession::set_error`: prefixes stored error with ❌ + timestamp
- `TaskSession::set_state`: clears `task.error` when `state.is_running()`

## Requirements coverage
- ✅ Add red X symbol: implemented as `\u{274C}` (❌)
- ✅ Add timestamp: implemented using fixed-offset timestamp and `"%Y-%m-%d %H:%M:%S"` + `"%z"`, matching existing formatting used elsewhere (see `zbobr-api/src/context/stage_title.rs::format_timestamp`).
- ⚠️ Clear error when task *enters* running: implementation currently clears error whenever `set_state` is called with any running state.

## Key issue (must fix)
### 1) `set_state` clears error on *every* running update, not only on entering running
**File:** `zbobr-dispatcher/src/task.rs`

Current code:
```rust
if state.is_running() {
    task.error = None;
}
```

`State::Running` encodes both pipeline and stage (`Running(Pipeline, Stage)`). `set_state` is likely called on stage transitions (e.g. `running:...:stage1` → `running:...:stage2`). With the current logic, any stage change while still running will wipe previously set errors, including errors that were set during the run.

**Task requirement wording** says “when task enters to running state, clear the error”, which implies a *transition* from non-running → running.

**Suggested fix:** clear only on the transition edge:
```rust
if !task.state.is_running() && state.is_running() {
    task.error = None;
}
```
This preserves errors during a run, but clears stale errors at the start of a new run.

## Consistency / coding standards notes (non-blocking)
### 2) Timestamp formatting logic is duplicated
The exact formatting `"%Y-%m-%d %H:%M:%S"` + `"%z"` already exists as `format_timestamp` in `zbobr-api/src/context/stage_title.rs` (currently private). The new code repeats this logic inline.

Given the project rule “Avoid repeated string literals”, consider centralizing timestamp formatting (e.g., a small public helper in `zbobr-api` or a dispatcher-local helper/const), so future changes can’t diverge.

### 3) Minor: could use a constant for the ❌ prefix
Not required, but if this symbol is reused elsewhere later, prefer a `const` to avoid accidental mismatch.

## Overall assessment
Good minimal change set and the timestamp matches existing context formatting. However, the current `set_state` behavior will clear errors too often and likely violates the “enters running” intent.

**Verdict:** changes require adjustment before merge.