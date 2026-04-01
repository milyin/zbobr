# Plan: Remove DeleteCtxRec Action + Hide ctx_rec_ for Records Without Links

## Two-part change

### Part 1: Remove DeleteCtxRec entirely

Remove the `delete_ctx_rec` MCP tool from all layers:
- `zbobr-api/src/config_tools.rs` — remove `DeleteCtxRec` variant from `McpTool` enum, string mapping, and static arrays
- `zbobr-dispatcher/src/mcp/common.rs` — remove `DeleteCtxRecParam` struct
- `zbobr-dispatcher/src/mcp/mod.rs` — remove re-export
- `zbobr-dispatcher/src/mcp/unified.rs` — remove `delete_ctx_rec` tool method
- `zbobr-dispatcher/src/mcp/traits.rs` — remove `delete_ctx_rec_impl`
- `zbobr-dispatcher/src/task.rs` — remove `delete_context_record` method
- `zbobr-api/src/task.rs` — remove `delete_record` from `TaskContext` (keep `find_record`/`find_record_mut`)
- `zbobr/src/init.rs` — remove `DeleteCtxRec` from all role `mcp` lists (planner, worker, test_planner, test_worker) and from the PLANNER_PROMPT instruction about `{mcp_delete_ctx_rec}`

### Part 2: Hide ctx_rec_N for records without a report_link

- `zbobr-api/src/context/mod.rs` (`MdRecord::fmt`): Only append `ctx_rec_N` when `report_link` is `Some`. Records without a link render as just `prefix + brief` with no ID tag.
- `zbobr-dispatcher/src/mcp/traits.rs` (`add_checklist_item_impl`): Only return `ctx_rec_{id}` in the response when the report was successfully stored (has a link).

### Tests to update
- `zbobr-dispatcher/tests/mcp_integration/abstract_scenarios.rs` — remove `delete_ctx_rec` step
- `zbobr-api/src/context/mod.rs` — update assertions that expect `ctx_rec_N` for linkless records