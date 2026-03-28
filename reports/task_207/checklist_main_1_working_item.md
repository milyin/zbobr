Both issues from the review:
1. stdout-only: executors only return stdout; stderr is discarded. Should combine stdout+stderr into the stored output.
2. not stored on errors: when process exits non-zero, `execute` returns `Err` and `execute_tool` sets `execution_output: None`.

Fix:
- Add `ExecutorOutput { output: String, exit_ok: bool }` to `zbobr-api/src/tool_executor.rs`
- Change `ToolExecutor::execute` return type to `anyhow::Result<ExecutorOutput>`
- Update all 3 executor implementations to: collect stdout+stderr, return `Ok(ExecutorOutput)` even on non-zero exit (exit_ok=false)
- Update `execute_tool` in `cli.rs` to use the new type, converting `exit_ok: false` into `execution_error`