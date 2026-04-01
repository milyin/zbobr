Review outcome: approved.

Assessment
- The branch removes the DeleteCtxRec operation consistently across the MCP surface: `McpTool`, tool-name lists, unified router definitions, MCP parameter exports, trait implementations, role defaults in `zbobr/src/init.rs`, and the integration scenario that enumerates available tools.
- I found no remaining `delete_ctx_rec` / `DeleteCtxRec` references in the repository, so the removal appears complete from the agent-facing API.
- The prompt-mode change is implemented in the right analog location: `zbobr-api/src/context/mod.rs` already owned the formatting split between normal rendering and prompt rendering, and the new logic extends that existing pattern instead of introducing ad-hoc filtering elsewhere.

Analog consistency
- The planner’s analog choice was appropriate: the existing `for_prompt` behavior in the markdown/context serializer is the correct place to decide what prompt-mode context exposes.
- The implementation follows the surrounding style: prompt-mode rendering remains a formatting concern inside `MdRecord::fmt`, while parsing remains focused on the persisted markdown form.
- Tool removal follows the same enum/router/config pattern already used for the other MCP tools, so the new code fits the existing architecture.

Correctness and standards review
- Prompt-mode IDs are now shown only for records the agent can still act on: checkboxes and records with report links. Non-interactive records without links no longer leak `ctx_rec_N` identifiers.
- Normal persisted markdown format is preserved, so context parsing for stored task data is unaffected.
- Tests cover both strengthened existing prompt-mode expectations and added targeted cases for mixed interactive/non-interactive records, which matches the task requirements.
- I did not find unrelated or unnecessary code changes in the branch.

Checklist status
- All checklist items in the provided task context were already marked complete; there were no unchecked items remaining to verify or update.

No review findings to fix.