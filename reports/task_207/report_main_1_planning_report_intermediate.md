# Plan: Capture Model Output and Link in Stage Title

## Goal
Collect all stdout from the AI model process, store it as a report file, and add `<sub>[output](url)</sub>` to the stage title.

## New Stage Title Format
Old: `pipeline:run_id:**stage** \`tool\` \`model\` <sub>[ts](prompt_url)</sub>`
New: `pipeline:run_id:**stage** \`tool\` \`model\` \`ts\` <sub>[prompt](url)</sub> <sub>[output](url)</sub>`

Changes:
- Timestamp moves from `<sub>[ts](url)</sub>` label into its own backtick token `` `ts` ``
- Prompt link becomes a separate `<sub>[prompt](url)</sub>` element
- New `<sub>[output](url)</sub>` element appended after prompt

## Steps

### 1. Add `output_link` to `StageInfo` (zbobr-api/src/task.rs)
Add `output_link: Option<String>` field alongside existing `prompt_link`, with the same `#[serde(default, skip_serializing_if = "Option::is_none")]` attribute.

### 2. Update `MdStageTitle` (zbobr-api/src/context/stage_title.rs)
- Add `output_link: Option<String>` field
- Update `Display`: emit timestamp as `` `YYYY-MM-DD HH:MM:SS +HHMM` ``, then `<sub>[prompt](url)</sub>` if present, then `<sub>[output](url)</sub>` if present
- Update `FromStr`: after parsing tool/model backticks, try a 3rd backtick as timestamp (new format); fall back to parsing `<sub>` with embedded timestamp (old format) for backward compatibility
- Update both `From` conversions to include `output_link`
- Update tests: add test for new format roundtrip, update display_format test

### 3. Change `ToolExecutor::execute()` return type (zbobr-api/src/tool_executor.rs)
Change return from `anyhow::Result<()>` to `anyhow::Result<String>` — the returned String is the collected stdout of the model process.

### 4. Update both executors to collect and return stdout
- `zbobr-executor-claude/src/lib.rs`: collect stdout lines (same pattern as existing stderr collection), join with newlines, return as Ok(output)
- `zbobr-executor-copilot/src/lib.rs`: same

### 5. Update dispatcher to store output and link it (zbobr-dispatcher/src/cli.rs)
- Update `SessionOutcome` to carry `output: Option<String>` from successful execution
- Update `execute_tool` to propagate the returned string into `SessionOutcome`
- After `execute_tool` succeeds, store output via `role_session.store_report()` using base_name `output_{pipeline_name}_{run_id}_{stage_name}`, then update `stage.info.output_link` via `modify_task` — mirrors the existing prompt storage pattern exactly

## Key Design Decisions
- **Analog**: Follows the prompt storage pattern verbatim (lines 468-486 in cli.rs): `store_report` → `modify_task` → set field on `stage.info`
- **Backward compat**: Parser supports both old format (`<sub>[ts](url)</sub>`) and new format (`` `ts` `` + separate `<sub>` elements), so existing task contexts remain readable
- **Output source**: stdout only — this is where the model's conversational output appears; stderr is error/warning noise already handled separately
