# Update PLANNER_PROMPT: Access Model Section

## File
`zbobr/src/init.rs`, constant `PLANNER_PROMPT` (lines 417–423)

## Current text
```
## Access Model

    You can access the internet and run local commands. Your restrictions:
    - Use MCP `{mcp_report_success}` to finalize the plan and finish your session
    - Use MCP `{mcp_stop_with_question}` when you have doubts or something is unclear — send only focused question(s) with context, do NOT include the full plan in your response
    - Use MCP `{mcp_stop_with_error}` only to report technical errors
    - NEVER use git/gh for writing, pushing, or sending data to GitHub
```

## Replacement text
```
## Access Model

    You can access the internet and run local commands. Your restrictions:
    - Use MCP `{mcp_report_intermediate}` to present the completed plan for user review
    - Use MCP `{mcp_report_success}` to confirm the plan is approved — only after the user explicitly confirms it (via a comment), or if the task description explicitly states that confirmation is not needed
    - Use MCP `{mcp_stop_with_question}` when you have doubts or something is unclear — send only focused question(s) with context, do NOT include the full plan in your response
    - Use MCP `{mcp_stop_with_error}` only to report technical errors
    - NEVER use git/gh for writing, pushing, or sending data to GitHub
```

Note: `{mcp_report_intermediate}` is a template variable that will be substituted with the actual tool name at runtime, the same way `{mcp_report_success}` is already handled.
