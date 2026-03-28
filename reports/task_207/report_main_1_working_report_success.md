# Implementation Summary

## Changes Made

### 1. Add `output_link` field to `StageInfo` and `MdStageTitle` (ctx_rec_3)
- Added `output_link: Option<String>` to `StageInfo` in `zbobr-api/src/task.rs`
- Added `output_link: Option<String>` to `MdStageTitle` in `zbobr-api/src/context/stage_title.rs`
- Updated both `From<&StageInfo>` and `From<MdStageTitle>` conversions
- Fixed all struct initializers in tests across the codebase

### 2. Update `MdStageTitle` format (ctx_rec_4)
New format:
```
main:1:**preparing** `copilot` `gpt-5-mini` `2026-03-27 13:54:35 +0100` <sub>[prompt](url)</sub> <sub>[output](url)</sub>
```

- Timestamp moved from `<sub>...</sub>` to backtick format (same as tool/model)
- Prompt link: `<sub>[prompt](url)</sub>` (separate element)
- Output link: `<sub>[output](url)</sub>` (separate element)
- Backwards-compatible parser: still reads old `<sub>timestamp</sub>` and `<sub>[timestamp](url)</sub>` formats
- Parser detects timestamp backtick by presence of spaces (tool/model names never contain spaces)

### 3. Change `ToolExecutor::execute` to return captured stdout (ctx_rec_5)
- Changed return type from `anyhow::Result<()>` to `anyhow::Result<String>`
- Updated all three executor implementations:
  - `zbobr-executor-claude`: captures stdout lines, returns joined string
  - `zbobr-executor-copilot`: same pattern
  - `zbobr-executor-mcp-tester`: uses existing `stdout_buf`, returns joined string
- `SessionOutcome` in `cli.rs` now carries `execution_output: Option<String>`

### 4. Store captured output as report and set `output_link` (ctx_rec_6)
- After `execute_tool` returns, store the output via `role_session.store_report()`
- Base name pattern: `output_{pipeline}_{run_id}_{stage}_end`
- Set `output_link` on the last stage in the task context
- Errors are logged as warnings (non-fatal) to not break the main flow

## All tests pass
