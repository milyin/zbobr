# Update PLANNER_PROMPT: Workflow Step 8 — Finalization

## File
`zbobr/src/init.rs`, constant `PLANNER_PROMPT` (line 444)

## Current text (step 8)
```
8. **Finish by calling `{mcp_report_success}`** with a brief rationale (why this approach was chosen, key design decisions, important constraints). Mention the chosen analog and why it's the right one to follow. Do NOT repeat the checklist items — the plan details are already captured there. This call finishes the session.
```

## Replacement text
```
8. **Present the plan by calling `{mcp_report_intermediate}`** with a brief rationale (why this approach was chosen, key design decisions, important constraints, chosen analog). Do NOT repeat the checklist items — the plan details are already captured there. Wait for the user to review.
9. **Finalize with `{mcp_report_success}`** only after the user explicitly confirms the plan (e.g., via a comment), OR if the task description explicitly states that confirmation is not needed.
```

Note: The existing step 8 becomes two steps (8 and 9). Renumber accordingly so the list remains sequential.
