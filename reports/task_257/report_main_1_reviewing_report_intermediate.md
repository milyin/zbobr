Reviewed `git diff origin/main...HEAD` and the branch commit history. The implementation matches the task requirements: prompt context now renders stage-name-only headers, removes prompt/output/timestamp/tool/model/link noise, filters empty stages in prompt mode, keeps plain `[ctx_rec_N]` references, and adds `get_ctx_rec` using the same dispatcher/MCP patterns as the existing `delete_ctx_rec` flow.

**Analog consistency**
- The new `get_ctx_rec` tool follows the existing MCP architecture well: `McpTool` enum wiring in `zbobr-api/src/config_tools.rs`, param type/parsing in `zbobr-dispatcher/src/mcp/common.rs`, shared implementation in `zbobr-dispatcher/src/mcp/traits.rs`, handler exposure in `zbobr-dispatcher/src/mcp/unified.rs`, and role exposure in `zbobr/src/init.rs`.
- The prompt-formatting changes are consistent with the existing `MdRecord` / `MdStage` / `MdContext` rendering model in `zbobr-api/src/context/mod.rs`, rather than introducing a parallel formatter.

**Code review assessment**
- I did not find blocking correctness issues in the implementation itself.
- I did not find unrelated/extraneous code changes in the branch diff.
- The implementation also addresses the previously reported regressions: prompt mode no longer emits stage markers, and non-prompt comment formatting remains distinct.

**Outstanding issue preventing final success report**
Some checklist items are still unchecked in the task context, so the checklist is not fully completed yet and this should not be marked as final review success:
- `ctx_rec_6` — "Simplify for-prompt context rendering in zbobr-api"
- `ctx_rec_17` — "Add get_ctx_rec step to MCP integration test scenario"

Later reports state that this work was implemented, so this appears to be a checklist-state mismatch rather than a code defect. Once those checklist items are marked complete, this review can be considered successful.