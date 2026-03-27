## Summary

Fixed the planner prompt to correctly implement the approval checking workflow as specified in the task requirements.

## Issues Fixed

1. **Step Reference Mismatch (Line 421)**
   - Changed: "call this only after creating checklist items (see workflow step 7)"
   - To: "call this only after creating checklist items (see workflow step 8)"
   - Rationale: Step 7 checks for approval, step 8 creates checklist items. The reference was pointing to the wrong step.

2. **Vague Approval Logic (Line 441)**
   - Changed: "Review the comments below to determine if the user has already approved this plan (or an earlier variant of it)"
   - To: "Review the most recent (last) comment below to determine if the user explicitly approves this plan"
   - Rationale: Task requirement specifies checking "the last comment" for approval. Removed ambiguous "earlier variant" language that allowed approval from any prior comment.

## Implementation Details

The fixed workflow (step 7) now clearly follows the task requirement:
- Check the most recent (last) comment for explicit user approval
- Check task description for "plan is preapproved" indication
- If approved → create checklist items (step 8) → call `report_success`
- If not approved → present plan → call `report_intermediate` (step 8.5)

This prevents creating checklist items that may need to be removed if the plan is rejected, eliminating unnecessary noise.

## Verification

- Changed file: `zbobr/src/init.rs` (PLANNER_PROMPT constant)
- Committed with: `fix: clarify planner prompt approval checking logic`
- Working directory: clean, all changes committed
