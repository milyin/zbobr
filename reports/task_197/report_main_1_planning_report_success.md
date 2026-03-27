# Plan Summary

## Approach
Two targeted changes in `zbobr/src/init.rs`:

1. **Add `ReportIntermediate` to planner MCP tools** — the tool is already defined and used by worker/reviewer; just adding it to the planner's vec.

2. **Update `PLANNER_PROMPT`** — three edits to the existing constant:
   - Access Model: add `report_intermediate` (present plan) and clarify `report_success` requires explicit user confirmation or task-level waiver
   - Workflow steps 3–4: shift from "explore codebase in detail" to "identify analog at module level, design architecture-level plan without code snippets"
   - Workflow step 8 → steps 8–9: split into "present with report_intermediate" then "confirm with report_success"

## Analog
The worker prompt's dual-finish pattern (`report_intermediate` for partial, `report_success` for complete) in the same file (lines 480–491) is the direct analog for the new planner confirmation flow.

## Key Constraints
- Template variable `{mcp_report_intermediate}` must be used (not the literal string), consistent with all other MCP references in the prompt.
- `ReportIntermediate` is already imported via the existing `use` statements in init.rs — no new imports needed.
- No other files need to change.
