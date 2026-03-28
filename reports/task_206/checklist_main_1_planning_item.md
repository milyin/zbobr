In `zbobr-dispatcher/src/task.rs`, modify the `set_error` method:

When `error` is `Some(msg)`, prepend a red X symbol and a timestamp before storing it. Use `chrono::Local::now()` (with fixed offset) to get the current time, formatted as `%Y-%m-%d %H:%M:%S %z` — the same format used for context timestamps elsewhere in the codebase.

The stored error string should look like: `❌ 2026-03-28 10:18:01 +0100: <original error message>`

When `error` is `None`, pass it through unchanged (no formatting needed).

This change is purely in the dispatcher's `set_error`, not in the backend trait, since timestamp generation is a dispatcher-level concern.