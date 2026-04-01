## Approach

Both `runner.run()` call sites in `cli.rs` need to catch errors and call `set_pause_with_status_and_signal(status, Signal::go(stage_name))` instead of propagating or just logging. This matches the existing worktree error-recovery pattern already used at lines 1596-1607.

The behavioral test uses an empty-description task to reliably trigger a pre-MCP error (the pre-flight check in `CliStageRunner::run()`), verifies the task ends up with `pause=true` and the correct resume signal after the first step, then verifies it converts to `State::Pause` with a stack entry on the next step.

## Chosen analog
Worktree error handling at lines 1596-1607 in `cli.rs` — same pattern of `format_error_status` + `set_pause_with_status_and_signal` + fallback `tracing::error!`.

## Key constraint
`apply_pause_to_state` reads `task.state.pipeline()` which returns `Some(p)` for both `Pending(p)` and `Running(p, _)`. So setting pause while state is still `Running` is safe — the pipeline is correctly extracted on the next iteration.
