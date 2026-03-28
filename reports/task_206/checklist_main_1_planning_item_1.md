In `zbobr-dispatcher/src/task.rs`, modify the `set_state` method:

When the new state is `State::Running(_, _)`, also set `task.error = None` inside the `modify_task` closure, before or after setting `task.state`.

This ensures that stale errors from previous attempts are cleared when a task starts running again, keeping the error section up-to-date.

Follow the same pattern already in place for `task.pause = true` when `task.confirm` is set — it's a side-effect applied conditionally based on the new state.