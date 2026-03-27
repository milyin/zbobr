# Checkboxes Should Be Subitems to Overview Sections

## Problem Statement
Currently, all context records (reports, checkboxes, comments) are rendered at the same indentation level in the context. However, checklist items (`add_checklist_item`) logically represent elaboration or sub-tasks of the overview/report they follow. They should be rendered as subitems (nested) under their parent report.

## Solution Architecture

### Closest Analog
The existing structure uses `MdStage` and `MdRecord` in `zbobr-api/src/context/mod.rs`. The pattern for hierarchical rendering is already established:
- Stage title (line 384)
- Indented records (line 386)

We'll extend this pattern to allow records to have child records:
- Report records at base indentation
- Checklist items nested under reports with extra indentation

### Key Changes

1. **Update MCP Tool Description** (zbobr-dispatcher/src/mcp/unified.rs)
   - Clarify that `add_checklist_item` creates items that are considered subitems/elaborations of reports
   - Description should reference the nesting behavior in the context

2. **Modify Context Rendering** (zbobr-api/src/context/mod.rs)
   - Change `MdStage.records` from `Vec<MdRecord>` to a structure that supports parent-child relationships
   - Modify `Display` impl for `MdStage` to render checkboxes nested under their preceding report record
   - Update parsing logic in `FromStr` for `MdStage` to correctly parse nested structure from markdown

3. **Update Markdown Format**
   - Base level records: `  - TYPE brief` (2 spaces indentation)
   - Nested checklist items: `    - [ ] brief` (4 spaces indentation)
   - This preserves markdown list semantics: a nested item naturally appears under its parent

### Design Principles
- Keep data semantics: checklist items follow the report they elaborate
- Non-breaking change: the markdown rendering is still valid markdown
- Parsing must be lenient: existing flat-structure markdown should still parse correctly
- Filtering in `list_tools` and other operations are unaffected

## Implementation Strategy

### Step 1: Update Tool Description
Change the `add_checklist_item` description to indicate items are subitems of reports.

### Step 2: Refactor MdStage.records
Introduce a structure that represents records with optional children:
```rust
// Option: Vector of record groups where each group = (parent record, vec of checkbox children)
// This maintains backward compat with existing code
```

### Step 3: Update Display Implementation
Modify `MdStage::fmt` to emit proper nesting:
- Parent report at 2 spaces: `  - TYPE brief`
- Child checkboxes at 4 spaces: `    - [ ] brief`

### Step 4: Update Parsing Logic
Modify `MdStage::from_str` to:
- Recognize checkbox records at 4+ space indentation as children of preceding report
- Handle existing flat structures (checkboxes at 2 spaces) as siblings for compatibility

### Step 5: Test and Validate
- Verify rendering produces correct markdown nesting
- Verify parsing handles both new nested format and legacy flat format
- No changes to MCP tool behavior or data model - only rendering changes
