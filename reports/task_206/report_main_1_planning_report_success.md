## Implementation Plan

Both changes are in `zbobr-dispatcher/src/task.rs`.

### set_error
When `error` is `Some(msg)`, format it with a ❌ prefix and a chrono timestamp before storing. Timestamp format matches context output: `%Y-%m-%d %H:%M:%S %z`.

### set_state
When new state is `State::Running(_, _)`, clear `task.error = None` inside the modify_task closure — same pattern as the existing `task.pause = true` side-effect.

### Why this approach
- All changes are at the dispatcher layer, which is the appropriate place for presentation-level formatting and state-transition side effects.
- No changes to the backend trait or API types needed.
- Follows existing patterns in the same function bodies.