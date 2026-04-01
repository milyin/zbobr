## Proposed Plan

### Context
Agents currently have a `delete_ctx_rec` MCP tool that lets them remove records from the context. The task removes this capability. Additionally, the prompt mode currently prints `[ctx_rec_N]` IDs for all records, even those the agent cannot interact with (non-checkbox records without a report link). Since those IDs are useless without an available operation, they should be omitted to reduce noise.

### Parsing Safety
The change is only to **prompt mode** rendering (`for_prompt=true`). Context is stored and parsed using `<sub>ctx_rec_N</sub>` format, which `FromStr` handles. Prompt-mode output is never parsed back. Therefore, omitting IDs in prompt mode does not affect context parsing — no changes to parsing needed.

### Changes

1. **Remove `DeleteCtxRec` from `McpTool` enum** (`zbobr-api/src/config_tools.rs`): Remove variant, `as_str()` arm, `FromStr` arm, `ALL_TOOLS`, `ALL_TOOL_NAMES`.

2. **Remove from role tool lists and prompt templates** (`zbobr/src/init.rs`): Remove `DeleteCtxRec` from all role mcp lists (planner, worker, test_planner, test_worker). Remove the `{mcp_delete_ctx_rec}` line from `PLANNER_PROMPT` (template variable would fail to resolve once the tool is removed from the allowed set).

3. **Remove MCP implementation** (`zbobr-dispatcher/src/mcp/unified.rs`, `traits.rs`, `common.rs`, `mod.rs`): Remove `delete_ctx_rec` tool function, `delete_ctx_rec_impl` method, `DeleteCtxRecParam` struct.

4. **Remove underlying delete operations** (`zbobr-dispatcher/src/task.rs`, `zbobr-api/src/task.rs`): Remove `delete_context_record` from session and `delete_record` + its test from `TaskContext`.

5. **Change prompt-mode rendering** (`zbobr-api/src/context/mod.rs`, `MdRecord::Display`): Only append `[ctx_rec_N]` when agent can interact: record is a checkbox (can call `check_checklist_item`) OR has a `report_link` (can call `get_ctx_rec`). All other records print just the brief text.

6. **Update tests**: Remove `delete_ctx_rec` scenario from integration tests; update prompt serialization tests to verify IDs are omitted for non-interactive records and retained for checkboxes/linked records.
