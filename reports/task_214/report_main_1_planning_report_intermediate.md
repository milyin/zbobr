# Plan: Clarify Planner Prompt Workflow

## Problem Statement
The planner agent currently creates checklist items before user approval, which causes "noise" when users reject a plan (items must be cleaned up). Additionally, planners don't reliably call `report_success` even after explicit user approval in comments.

## Desired Behavior Change
The planner workflow should:
1. **Generate a plan** based on task description, earlier variants, and user comments
2. **Check for approval** in the last user comment or in the task description
3. **Branch based on approval status:**
   - If approved → create checklist items AND use `report_success` to proceed with implementation
   - If not approved → use `report_intermediate` to present plan for user review (checklist items created only after approval)

## Root Cause Analysis
The analog for this change is the Worker prompt (lines 448-497 in init.rs), which already has a clear decision pattern: it checks task status and branches based on context.

The current planner prompt (lines 411-446 in init.rs) has this workflow:
- Step 6: Determine if plan is clear
- Step 7: Create checklist items (always)
- Step 8: Report with `report_intermediate`
- Step 9: Wait for approval, then call `report_success`

This always creates items before approval, and leaves the responsibility to the next agent call to finish with `report_success`.

## Implementation Approach

### Changes to planner prompt (init.rs lines 411-446):

1. **Before step 6 (in Access Model or new section):** Add explicit instructions to check for approval:
   - Scan the context (task description and last comment) for approval signals
   - Approval signals: "approved" / "looks good" / "proceed" / task description explicitly stating plan is "preapproved"
   - Store this decision to use in branching logic

2. **Restructure steps 6-9 workflow:**
   - Current step 6 (plan clarity check): Keep as-is
   - New step: After plan is clear, **check for user approval** (skip this step if task explicitly pre-approves)
   - **Branch:**
     - **Path A (Plan approved):** Create checklist items → call `report_success` with plan details (finish)
     - **Path B (Plan not approved):** Call `report_intermediate` with plan for review (finish, await user comment)

3. **Key changes to step instructions:**
   - Rename/renumber step 7: Make approval check explicit before checklist creation
   - Merge old steps 7-9 into conditional branches:
     - "If approved: ... prepare checklist items ... call `report_success`"
     - "If not approved: ... call `report_intermediate` ... wait for approval"

4. **Note:** Keep the "don't mention pipeline workflow" constraint (CLAUDE.md rule) — don't change anything about stage names or transitions in the prompt.

## Design Rationale

- **Follows existing pattern:** Worker prompt already branches on context (check status → decide next action)
- **Reduces noise:** Checklist items only created after approval avoids cleanup burden
- **Fixes approval issue:** Explicit approval check + decision tree ensures `report_success` is called when appropriate
- **Clear decision point:** Makes the planner's approval detection logic transparent and debuggable

## Files to Modify

- `/data/home/skynet/zdam/zbobr-dev/workspaces/task-214/zbobr/zbobr/src/init.rs` — PLANNER_PROMPT constant (lines 411-446)

## Constraints

- Do not modify Worker or Reviewer prompts
- Do not add pipeline workflow details
- Keep the prompt concise and actionable
- Preserve existing step numbering style or clearly indicate new structure
