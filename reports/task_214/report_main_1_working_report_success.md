## Task: Clarify Planner Prompt

The planner prompt workflow has been successfully updated to implement the approval-checking logic as specified in the task requirements.

### Changes Made

The `PLANNER_PROMPT` in `zbobr/src/init.rs` has been modified to:

1. **Separate approval checking from checklist creation**: Step 7 now explicitly checks for user approval BEFORE proceeding to create checklist items

2. **Approval detection logic**: The planner now reviews:
   - Comments to determine if the user has already approved the plan
   - Task description to check for explicit preapproval statements (e.g., "plan is preapproved")

3. **Conditional workflow paths**:
   - **If approved**: Proceed to create checklist items (step 8), then call `report_success` to finalize and proceed to implementation
   - **If not approved**: Present the plan via `report_intermediate` (step 8.5) and wait for user feedback WITHOUT creating checklist items

4. **Noise reduction**: Checklist items are only created when the plan is explicitly approved, avoiding the need to remove rejected items

### Key Behavioral Changes

- **report_intermediate**: Now called when presenting a plan for review (only when plan is not yet approved)
- **report_success**: Now called only after creating checklist items (when plan is approved), replacing the earlier practice of calling it upon approval

### Benefits

- Eliminates noise from creating checkboxes that may need removal if a plan is rejected
- Creates a clearer workflow that distinguishes between planning and implementation preparation
- Makes it explicit when the planner transitions from "design mode" to "execution mode"
- Ensures checklist items are only created for approved plans, providing a more structured pipeline

The implementation follows the specified requirements from the task description and aligns with the workflow described in the CLAUDE.md project rules.