# Implementation: Clarify Planner Prompt Workflow

## What was done

Modified the `PLANNER_PROMPT` constant in `zbobr/src/init.rs` (lines 411-446) to implement the approval-aware workflow:

### Changes made:

1. **Opening description** (line 413): Removed reference to preparing checklist items upfront, focusing instead on designing the plan
2. **Access Model section** (lines 420-422): Clarified when to use `report_intermediate` (only when plan not yet approved) vs `report_success` (after creating checklist items)
3. **New approval-checking step** (lines 439-450): Added Step 7 to explicitly check for user approval in comments or task description
4. **Conditional branching**: If approved → create checklist items and call `report_success`; if not → present plan with `report_intermediate`
5. **Reorganized final steps**: Renamed old step 7 to step 8, added step 8.5 for presenting unapproved plans

## Why this approach

This addresses the core issue: planners were creating checklist items before user approval, causing noise if plans were rejected. The new workflow ensures:
- Plans are reviewed and approved before checklist items are created
- Checklist items only exist for approved work
- Planners properly finalize with `report_success` after user approval

## Verification

The implementation follows the same pattern used in the Worker prompt and fully implements the requirements specified in the task description.