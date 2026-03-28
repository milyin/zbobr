
## What to change

In `zbobr-dispatcher/src/task.rs` (`RoleSession` impl):
- Remove `set_error` method (replaced by `set_status`)
- Remove `set_pause(bool)` (or restrict to `false` only)
- Remove `set_pause_with_signal(Signal)` — replace with `pause_with_signal_and_status(Signal, icon, message)`
- Add corresponding methods that delegate to `modify_task` following the same pattern as the existing `set_pause_with_signal`

The new methods must be available on `RoleSession` so that `mcp/traits.rs` and `cli.rs` can call them.

## Why

`RoleSession` mirrors a subset of `TaskMut` operations. Since we're removing bare `set_pause(true)` from the trait, the session-level methods must be updated to match.

## Analog

Follow the exact same pattern as the existing `set_pause_with_signal` in `zbobr-dispatcher/src/task.rs` — it already atomically sets `pause=true` + `signal` in one `modify_task` call. The new methods just add `status` to the same pattern.
