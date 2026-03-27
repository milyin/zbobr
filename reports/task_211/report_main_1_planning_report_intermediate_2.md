# Implementation Plan: Checkboxes as Subitems of Reports

## Overview
Make checklist items (checkboxes) semantically and structurally subordinate to their parent report records (Success/Failure). Checkboxes will be elaborations of the report's work items.

## Design Approach

### Analogy
The pattern follows the existing relationship model in the codebase where `StageContext` contains `ContextRecord`s with metadata. We're extending this by adding a parent-child relationship within the records list itself.

### Key Design Decision
Add an optional `parent_record_id` field to `ContextRecord` to establish the hierarchy. This is backward-compatible and follows the existing pattern of optional, serializable fields in the struct.

## Implementation Steps

### 1. Update MCP Tool Description (ctx_rec_4)
Clarify in the `add_checklist_item` tool description that checklist items are elaborations of reports. This sets expectations for agent behavior.

### 2. Extend ContextRecord Data Structure (ctx_rec_5)
Add `parent_record_id: Option<u64>` field to `ContextRecord` in `zbobr-api/src/task.rs`. This field references the id of the parent report record. Backward-compatible with default/skip_serializing_if attributes.

### 3. Update add_checklist_item_impl Logic (ctx_rec_6)
When a checklist item is added, find the most recent Success/Failure record and use its id as the parent_record_id. Handles the edge case where no report exists yet (parent_record_id = None).

### 4. Update add_checkbox_record Signature (ctx_rec_7)
Add `parent_record_id: Option<u64>` parameter to the method that creates checkbox records, so the parent information flows through the entire call chain.

### 5. Update Rendering/Display Logic (ctx_rec_8)
Modify the code that formats StageContext for output to nest checklist items under their parent reports, creating the visual hierarchy in the context display.

## Edge Case Handling
If a checklist item is added before any report exists, it stores parent_record_id = None. This allows orphaned items to exist temporarily and be associated with the next created report if needed.

## Why This Approach
- **Semantic clarity**: The tool description now explicitly states the parent-child relationship
- **Data model integrity**: The structure captures the hierarchy explicitly rather than relying on display logic alone
- **Backward compatible**: Optional fields and serialization skip rules maintain compatibility
- **Scalable**: Future enhancements can leverage the parent_record_id field for queries, filtering, or complex hierarchies

## Files to Modify
- `zbobr-dispatcher/src/mcp/unified.rs` - tool description
- `zbobr-api/src/task.rs` - ContextRecord struct, add_checkbox_record method
- `zbobr-dispatcher/src/mcp/traits.rs` - add_checklist_item_impl implementation
- Task backend(s) - add_checkbox_record implementation (fs or github)
- Context rendering/display code - formatting for hierarchy

## Implementation Work Items
See the checklist items (ctx_rec_4 through ctx_rec_8) for detailed implementation steps.