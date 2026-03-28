## Changes

Modified `zbobr-dispatcher/src/task.rs`:

1. **`set_error`**: When a non-None error is provided, prepends `❌ {timestamp} ` to the message. Timestamp uses `chrono::Utc::now().with_timezone(&self.zbobr.config().fixed_offset())` formatted as `YYYY-MM-DD HH:MM:SS +HHMM` — same pattern as `cli.rs`.

2. **`set_state`**: Added `if state.is_running() { task.error = None; }` before setting the state, so entering Running clears any prior error.

Build verified clean. Commit: cc294c4