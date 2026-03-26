## Summary

Implemented all missing pieces identified in the review:

### MCP Tools (5 commits)

1. **Removed**: `GetHistory`, `GetFullReport` from `McpTool` enum and all implementations
2. **Added**: `AddChecklistItem` - adds checkbox context record, with optional `long_description` stored as a file
3. **Added**: `CheckChecklistItem` - marks a checkbox context record as checked (accepts numeric id or `ctx_rec_N` string)
4. **Added**: `DeleteCtxRec` - deletes any context record by id (accepts numeric id or `ctx_rec_N` string)
5. **Added**: `store_report` to `TaskMut` trait for direct report file storage (used by `AddChecklistItem` long descriptions)

### RoleSession Context Methods
- `add_checkbox_record()` - adds unchecked checkbox to last stage
- `check_checkbox_record()` - marks checkbox as checked
- `delete_context_record()` - deletes any record by id
- `add_context_record()` - adds any record type to last stage
- `store_report()` - stores report file via backend

### Report Storage Change
Reports (`report_success`, `report_failure`, `report_intermediate`) now store results as context records instead of comments, per task requirement "comments should not be posted at all, stages results are stored in context only".

### Context in Prompts
- Added `{context}` template variable that serializes `TaskContext` with `for_prompt=true` (omits prompt links) and interspersed user comments
- Added `# Context` section with `{context}` placeholder to `TASK_TEMPLATE`

### Stage Creation
- `CliStageRunner` now creates a `StageContext` with pipeline, run_id, stage name, tool, model, and timestamp when entering a new stage

### Prompt Updates
- Removed all `{mcp_get_history}`, `{mcp_get_checklist}`, `{mcp_delete_checklist_item}` references
- Updated planner prompt: uses `{mcp_add_checklist_item}` and `{mcp_delete_ctx_rec}`
- Updated worker prompt: uses `{mcp_check_checklist_item}`, `{mcp_add_checklist_item}`, `{mcp_delete_ctx_rec}`
- Reviewer/tester: no checklist tools (per task requirement), can send multiple success/failure reports
- All prompts reference context section instead of history

### Role MCP Tool Assignments
- **Planner**: StopWithError, StopWithQuestion, ReportSuccess, AddChecklistItem, DeleteCtxRec
- **Worker**: StopWithError, ReportSuccess/Failure/Intermediate, StopWithQuestion, AddChecklistItem, CheckChecklistItem, DeleteCtxRec
- **Reviewer**: StopWithError, ReportSuccess/Failure/Intermediate, StopWithQuestion (no checklist tools)
- **Tester**: StopWithError, ReportSuccess/Failure, StopWithQuestion (no checklist tools)
- **Merger**: StopWithError, ReportSuccess, StopWithQuestion

### Tests
- Updated `report_success_stores_context_records` test (was `report_success_posts_comment_to_backend`)
- Updated `all_mcp_tools_scenario` integration test for new tool names and parameters
- Removed dead code (`get_history_for_run`)
- All 109 tests pass, clean build with no warnings