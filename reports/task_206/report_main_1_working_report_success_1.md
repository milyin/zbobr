## Fix: Clear error only on transition into Running state

### Problem
The reviewer found that `set_state` was clearing `task.error` whenever the state was set to Running, even if the task was already in the Running state. This was too aggressive.

### Fix
Changed the condition in `zbobr-dispatcher/src/task.rs` `set_state` from:
```rust
if state.is_running() {
    task.error = None;
}
```
to:
```rust
if task.state != state && state.is_running() {
    task.error = None;
}
```

Now the error is only cleared when the task **transitions into** the Running state (previous state differs from new state), not on every call to `set_state` with Running.

### Commit
`f2a3f46` fix: clear error only on transition into Running state
