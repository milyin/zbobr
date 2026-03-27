# Implementation Plan: Checkboxes as Subitems to Overview Sections

## Overview
The task requires making checkbox items render as indented subitems of their parent report (overview/summary). Currently, checkboxes are rendered as flat list items within a stage. They should instead be nested under the report they elaboration on.

## Architecture & Design

### Chosen Analog
The existing markdown serialization system (zbobr-api/src/context/mod.rs) is the analog. The MdStage Display trait (line 382-389) already renders records with indentation. We will extend this pattern to support multi-level nesting.

### Key Changes

1. **Data Model (zbobr-api/src/task.rs)**
   - Add `parent_record_id: Option<u64>` field to `ContextRecord` struct to track parent-child relationships
   - This allows checkboxes to reference their parent report without changing the flat storage in StageContext

2. **Checklist Item Creation (zbobr-dispatcher/src/mcp/traits.rs + task.rs)**
   - Update `add_checklist_item_impl` to find the last non-checkbox report in the current stage and set it as parent
   - Handle edge case: if no parent report exists yet (checkboxes created before first report), set parent_record_id to None; the report will adopt them when created
   - Update `add_checkbox_record` method signature to accept optional parent_record_id parameter

3. **Markdown Rendering (zbobr-api/src/context/mod.rs)**
   - Modify `MdStage::Display` to detect parent-child relationships and render checkboxes with additional indentation (4 spaces instead of 2)
   - Checkboxes with a parent_record_id render after their parent report with deeper indentation
   - Checkboxes without a parent render at normal level (backward compatible)

4. **Serialization/Deserialization (zbobr-api/src/context/mod.rs)**
   - Update MdRecord parsing to preserve/ignore parent_record_id (may be implicit from context structure)
   - Update MdRecord serialization to include parent relationship information if needed

5. **Tool Description (zbobr-dispatcher/src/mcp/unified.rs or wherever MCP tool descriptions are defined)**
   - Update the `add_checklist_item` tool description to clarify: "Checklist items are subitems/elaborations of the current overview/report and will be rendered as nested items under that report"

## Implementation Notes

- The parent-child relationship is stored in the record itself, not in the stage structure, preserving the flat vec-based storage
- Rendering logic in Display implementations can query records to build the hierarchy
- No database migrations needed; parent_record_id is optional and defaults to None
- Backward compatible: checkboxes without a parent render normally

## Testing Considerations

- Verify checkboxes added after a report are associated with that report
- Test edge case: checkboxes created before any report (parent_record_id = None)
- Verify markdown roundtrip: serialize and deserialize maintains relationships
- Check indentation in markdown output (should show clear nesting with 4-space indent for subitems)