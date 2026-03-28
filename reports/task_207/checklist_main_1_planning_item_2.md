Change the `ToolExecutor` trait in `zbobr-api/src/tool_executor.rs`:
- Return type changes from `anyhow::Result<()>` to `anyhow::Result<String>`, where the `String` is the captured stdout of the model process.

Update all executor implementations to collect stdout and return it:

**`zbobr-executor-claude/src/lib.rs`**: currently stdout lines are only logged. Collect them into a `Vec<String>` (similar to how stderr is already collected), join them with newlines, and return the joined string.

**`zbobr-executor-copilot/src/lib.rs`**: apply the same pattern — collect stdout, return it.

**`zbobr-executor-mcp-tester/src/lib.rs`**: return `Ok(String::new())` since it has no real process output.

In all executors, on error (`!status.success()`), still bail with the existing error message. The stdout collected up to that point can be included in the error context if useful, but is not required.

Analog: the stderr collection pattern already used in the Claude executor.