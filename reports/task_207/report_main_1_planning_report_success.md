# Implementation Plan: Capture Model Output

## Approach

Mirror the existing `prompt_link` pattern throughout the stack. The prompt is saved before execution and linked in the stage title; the output will be saved after execution the same way.

## Key Design Decisions

1. **Format change**: Timestamp moves from `<sub>` to backtick (like tool/model), prompt and output become separate `<sub>[label](url)</sub>` elements. This is cleaner and extensible — adding more links is trivial.

2. **Backward compatibility**: The `FromStr` parser will handle both old and new timestamp formats so existing stored contexts parse correctly.

3. **Return type change**: `ToolExecutor::execute` returns `anyhow::Result<String>` (captured stdout). This is the minimal interface change — the caller in `cli.rs` gets the output directly without needing shared state.

4. **Analog**: The prompt saving pattern (lines 468–486 in `cli.rs`) is the direct analog for saving the output after execution.

## Checklist Summary
1. Add `output_link` field to `StageInfo` and `MdStageTitle`
2. Update `MdStageTitle` format (Display + FromStr + tests)
3. Change `ToolExecutor::execute` return type; update all executor impls to collect/return stdout
4. Capture output in `SessionOutcome`, save as report, set `output_link` in stage after execution