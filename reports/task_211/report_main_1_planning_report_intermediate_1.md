# Implementation Plan: Checklist Items as Subitems of Reports

## Overview
The task requires that checklist items (created via `add_checklist_item`) appear as nested subitems under report/overview records rather than as flat sibling items. This provides a hierarchical structure where reports serve as containers for related checklist items.

## Architecture & Design

### 1. **Closest Analog**: Stage-Record Relationship
The existing codebase already implements a hierarchical structure: stages contain records, and records are rendered with indentation under the stage title. We apply the same pattern one level deeper: reports can contain child records.

**Current structure:**
```
- Stage title
  - Record 1 (flat list)
  - Record 2 (flat list)
```

**Desired structure:**
```
- Stage title
  - Report/Overview record
    - Checkbox subitem 1 (nested)
    - Checkbox subitem 2 (nested)
```

### 2. **Core Changes Needed**

#### 2.1 Domain Model Changes (`zbobr-api/src/task.rs`)
- **Add child records support**: Extend `ContextRecord` to include an optional `children: Vec<ContextRecord>` field
- This allows records to have a tree structure while maintaining backward compatibility (existing records have empty children)

#### 2.2 Markdown Serialization (`zbobr-api/src/context/mod.rs`)
- **Update `MdRecord` rendering**: Modify `fmt::Display` to emit nested records with increased indentation
  - Parent record: `  - ✅ brief <sub>ctx_rec_1</sub>`
  - Child records: `    - [ ] child 1 <sub>ctx_rec_2</sub>`
- **Update parsing**: Modify `FromStr` to recognize indentation levels and reconstruct the tree structure
  - Detect deeper indentation (4 spaces + record marker) as child records
  - Group children until indentation returns to parent level

#### 2.3 Context Tree Construction Logic
- **Report-to-checkbox grouping**: After parsing/deserializing context, implement logic to reorganize flat records into a tree:
  - When a report record (Success/Failure/Comment) is followed by checkboxes, move the checkboxes as children
  - Handle edge case: checkboxes created before any report → attach to the next report that arrives
- **Location**: Add helper functions in `zbobr-api/src/context/mod.rs` or `zbobr-api/src/task.rs` to normalize the flat record list into a tree structure

#### 2.4 MCP Tool Descriptions
- **Update `AddChecklistItemParam` schema description** in `zbobr-dispatcher/src/mcp/common.rs`:
  - Clarify that "checklist items are elaborations of the report provided" 
  - Explain that they appear as subitems under the most recent report/overview section

### 3. **Edge Cases & Rules**

1. **Checkboxes before first report**: Store them in a temporary "pending" state; when a report arrives, attach them as children
2. **Multiple reports in sequence**: Checkboxes follow the "most recent report" rule  
3. **Backward compatibility**: Old contexts with flat records should parse correctly and be automatically normalized to tree structure

### 4. **Implementation Order**

1. Modify `ContextRecord` to support children (add optional field)
2. Update markdown rendering (`MdRecord` display and parsing) to handle indentation
3. Add tree-normalization logic to group checkboxes under reports
4. Update MCP tool schema descriptions  
5. Test: verify that old flat records are read correctly, new nested records render and parse correctly
6. Edge case handling: test checkbox-before-report scenarios

## Key Design Principles

- **Minimal structural change**: Records remain mostly the same; children are optional and backward-compatible
- **Indentation-based**: Leverage existing markdown indentation to signal parent-child relationships (4-space indent for children)
- **Lazy normalization**: Can normalize the tree during deserialization or lazily when needed
- **Preserve behavior for non-checkbox records**: Only checkboxes are typically nested; success/failure/comment records can have children but this is the primary use case for checkboxes
