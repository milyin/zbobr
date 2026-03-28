# Fix: capture stdout+stderr, store output even on process failure

## Issues fixed from review

1. **stdout-only capture**: Executors now collect both stdout and stderr, combining them with a `--- stderr ---` separator when stderr is non-empty.

2. **Not stored on errors**: Changed `ToolExecutor::execute` return type from `anyhow::Result<String>` to `anyhow::Result<ExecutorOutput>` where `ExecutorOutput { output: String, exit_ok: bool }`. Executors now return `Ok(ExecutorOutput { ..., exit_ok: false })` on non-zero process exit instead of `Err`. The `execute_tool` function in `cli.rs` maps `exit_ok: false` to `execution_error` while still populating `execution_output`, so the output is always stored.

3. **Repeated literals**: Added `PROMPT_LABEL` and `OUTPUT_LABEL` constants in `stage_title.rs` and replaced the hardcoded `"prompt"` and `"output"` string literals in both Display and FromStr with these constants.

## Changes

- `zbobr-api/src/tool_executor.rs`: Added `ExecutorOutput { output, exit_ok }` struct; changed trait signature
- `zbobr-api/src/lib.rs`: Re-exported `ExecutorOutput`
- `zbobr-api/src/context/stage_title.rs`: Added `PROMPT_LABEL`/`OUTPUT_LABEL` constants
- `zbobr-executor-claude/src/lib.rs`: Updated to collect stdout+stderr, return `ExecutorOutput`
- `zbobr-executor-copilot/src/lib.rs`: Same
- `zbobr-executor-mcp-tester/src/lib.rs`: Same
- `zbobr-dispatcher/src/cli.rs`: Updated `execute_tool` to handle `ExecutorOutput`

All tests pass.