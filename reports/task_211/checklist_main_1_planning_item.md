# Update add_checklist_item MCP Tool Description

## What to change
Update the description in `zbobr-dispatcher/src/mcp/unified.rs` at the `add_checklist_item` tool definition (around line 127-129).

## Current description
"Add a new unchecked checklist item to the current stage context. Brief summary is stored as context record text; full report is stored as a file."

## New description
Should clarify that checklist items are elaborations of reports. Suggested wording:
"Add a new unchecked checklist item to the current stage context as an elaboration of the most recent report. The checklist items are considered as elaboration of the report provided. Brief summary is stored as context record text; full report is stored as a file."

## Why
The tool description should make it clear to users (agents) that checklist items have a semantic relationship to reports - they're not standalone items, but rather detailed breakdowns or elaborations of the report's work items.

## How to apply
- Find the `#[tool(...)]` macro for `add_checklist_item` in unified.rs
- Update only the description text
- Ensure the updated description explains the parent-child relationship between checklist items and reports