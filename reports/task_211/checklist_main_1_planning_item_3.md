# Update add_checkbox_record Method

## What to change
Check the `add_checkbox_record` method in the RoleSession/task backend and update if necessary to accept a parent_record_id parameter.

## Current signature
You'll need to find where `add_checkbox_record` is implemented. It's likely in:
- `zbobr-api/src/task.rs` (RoleSession impl)
- Or in a backend implementation (zbobr-task-backend-fs or zbobr-task-backend-github)

The current signature takes: `brief: String, report_link: Option<String>`

## New signature needed
Should accept an additional parameter: `parent_record_id: Option<u64>`

## Why
To pass the parent report id information from add_checklist_item_impl down to the actual record creation logic.

## How to apply
- Find the add_checkbox_record method definition
- Add the `parent_record_id: Option<u64>` parameter
- Use this parameter when constructing the ContextRecord
- Ensure the record's parent_record_id field is set correctly
- Update all callers of this method (probably just add_checklist_item_impl)