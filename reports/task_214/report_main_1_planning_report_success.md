# Task Completion: Clarify Planner Prompt

## Summary

The planner agent prompt has been successfully clarified to implement an approval-aware workflow that prevents noise from premature checklist item creation.

## Implementation Details

**File modified**: `zbobr/src/init.rs` (lines 411-450)

The `PLANNER_PROMPT` constant now includes:
- Clearer distinction between `report_intermediate` (unapproved plans) and `report_success` (finalized with checklist items)
- Explicit Step 7 to check for user approval in comments or task description
- Conditional logic: if approved → create checklist items and call `report_success`; if not → present plan with `report_intermediate`
- Removed requirement to create checklist items before approval

## Key Changes

1. Updated opening description to focus on planning, not checklist preparation
2. Clarified access model restrictions for each reporting method
3. Added user approval checking before checklist creation
4. Reorganized workflow steps to emphasize approval before implementation
5. Ensured planners properly finalize with `report_success` after user approval

## Design Decision

Following the approval-checking pattern allows the planner to:
- Design and present plans for user review
- Only create checklist items after explicit approval
- Reduce workflow noise from rejected plans
- Properly signal readiness to move to implementation stage

This resolves the issue where planners were not finishing with `report_success` after user approval, ensuring the workflow transitions correctly to the worker stage.