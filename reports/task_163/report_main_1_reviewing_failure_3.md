# Review Report: Task 163 - Context Structure

The implementation of `TaskContext` structure and parsing logic is correct, but the integration with MCP tools and prompt templating is incomplete.

## Verified Fixes
- **Data Structures**: `TaskContext` correctly replaces `checklist`. `StageContext` no longer has `user_comment`.
- **Parsing Logic**: `Pipeline::from` usage is safe (wraps `FromStr` which is infallible for `Pipeline`). `parse_record_line` correctly errors on unrecognized lines via `parse_context` control flow.
- **Obsolete Code Removal**: `ChecklistItem` and `checklist_format.rs` are removed.

## Issues Found

### 1. Missing and Incorrect MCP Tools
The task description explicitly required:
- Remove `GetHistory` (It is still present in `zbobr-dispatcher/src/mcp/unified.rs` and `zbobr-api/src/config_tools.rs`).
- Add `DeleteCtxRec` (Missing from `McpTool` enum and implementation).
- Method `AddChecklistItem` should accept optional long description (Missing from `McpTool` enum and implementation).

Currently, the `Checklist` related tools were removed, but their replacements (`DeleteCtxRec`, `AddChecklistItem` for context) were **not added**. This leaves the agent with no way to modify the context (add checkboxes/questions or delete records), except via `report_success`/`report_failure`.

### 2. Missing Context in Prompts
The task required:
- "Add placeholder {context} which inserts md formatted `TaskContext` to the output."

In `zbobr-dispatcher/src/prompts.rs`, the `build_template_variables` function adds `title`, `description`, `last_report`, etc., but **does not add `{context}`**. The `serialize_context` function is implemented in `zbobr-api` but is never called during prompt generation.
As a result, agents will not see the task context in their prompts.

## Required Actions
1.  **Update MCP Tools**:
    - Remove `GetHistory` from `UnifiedMcp` and `McpTool` enum.
    - Add `DeleteCtxRec` tool (accepts `id` or `ctx_rec_id`).
    - Add `AddChecklistItem` tool (or `AddContextRecord`) that adds a `ContextRecord` of type `Checkbox`. It should accept `brief` and optional `long_description` (stored to file).
2.  **Update Prompt Builder**:
    - In `zbobr-dispatcher/src/prompts.rs`, modify `build_template_variables` (or `build_full_prompt`) to call `serialize_context(..., for_prompt=true)` and insert the result as the `{context}` variable.
