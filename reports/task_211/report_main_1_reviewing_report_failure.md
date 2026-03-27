## Scope reviewed
Diff vs `origin/main...HEAD` (6 files):
- `zbobr-api/src/task.rs`: add `ContextRecord.parent_record_id: Option<u64>`
- `zbobr-api/src/context/mod.rs`: hierarchical markdown render/parse using `parent_record_id`
- `zbobr-dispatcher/src/mcp/traits.rs`: `add_checklist_item_impl` chooses parent as most recent report
- `zbobr-dispatcher/src/task.rs`: `add_checkbox_record(..., parent_record_id)`
- `zbobr-dispatcher/src/mcp/unified.rs`: MCP tool description updated
- `zbobr-task-backend-github/src/separator.rs`: test fixture updates

## Primary requirement check
**Task requirement:** “checkboxes should be always subitems to the overview sections” and **explicit edge case**: “when checkboxes are created before any top-level report is created… place the checkboxes under the next report”.

### Finding (must-fix)
Current behavior does **not** satisfy the edge case:
- `add_checklist_item_impl` computes `parent_record_id` as the most recent report record (Success/Failure/Comment) in the last stage.
- If **no report exists yet**, it sets `parent_record_id = None`.
- `MdStage::fmt` renders **only** records with `parent_record_id.is_none()` as top-level, so these “early” checkboxes remain **top-level list items**, not nested under the next report.

This directly contradicts the explicit plan note and the typical usage pattern (create checklist items during planning, then `report_intermediate` afterward).

## Analog / consistency
The change follows the existing markdown codec pattern in `zbobr-api/src/context/mod.rs` (Display/FromStr roundtripping via indentation), but the parent assignment strategy is inconsistent with the requirement: it attaches checkboxes to the **previous** report (when present) and leaves pre-report checkboxes unparented.

## Additional robustness observations
1) **Rendering drops records with invalid parents**
`MdStage::fmt` never prints records whose `parent_record_id` is `Some(_)` unless a top-level record with that `id` is printed. If a parent id is missing/mismatched (partial updates, manual edits, cross-stage refs), the record disappears from markdown output.
- Suggestion: add a fallback path (e.g., treat “dangling-parent” records as top-level) or validate parent existence.

2) **Markdown parse parent inference depends on last top-level id**
`MdStage::from_str` assigns child parent based on indentation and `last_top_level_id`. If a checkbox is top-level (current edge case), subsequent indented lines could attach to a checkbox parent incorrectly.

## Suggested fix approach
Prefer **making the data correct** (assign parents) rather than only changing rendering:
- When a report record is created (`report_impl` / `RoleSession::add_context_record` for Success/Failure/Comment), if there are any checkbox records in the current stage with `parent_record_id == None` that were created before the first report, set their `parent_record_id = Some(new_report_id)`.
  - Minimal: after adding the report record, scan the current stage and reparent all checkbox records with `parent_record_id == None` (since requirement says checkboxes should always be subitems).
  - More precise: only reparent the “prefix” orphan checkboxes that appear before the first report.

Alternative (less ideal): implement “pending checkbox” buffering in markdown rendering (treat top-level checkboxes as belonging to the next report), but that keeps JSON context ambiguous and doesn’t enforce the invariant that checkboxes are always nested.

## Conclusion
The branch introduces the structural capability (`parent_record_id`) and hierarchical rendering, but it **misses the required edge-case behavior** for pre-report checkboxes. This is a functional mismatch with the task description and must be corrected before approval.