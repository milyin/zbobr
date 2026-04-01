# Plan approved and checklist created

## Approach

Two-part change:

**Part 1 — Remove DeleteCtxRec:** Strip the enum variant, role assignments, MCP handler, impl method, and session method across 7 files. `parse_ctx_rec_id` and `TaskContext::delete_record` are kept because they are used by other tools/tests.

**Part 2 — Suppress non-interactive IDs in prompt mode:** Change `MdRecord::fmt()` in context/mod.rs so that `[ctx_rec_N]` is only printed when `for_prompt && (is_checkbox || has_report_link)`. Storage format and parsing are unaffected.

## Key design decisions
- `parse_ctx_rec_id` retained — still needed by `get_ctx_rec_impl` and `check_checklist_item_impl`
- `TaskContext::delete_record` retained — domain-layer method with its own test, not in scope to remove
- Interactive record criteria: Checkbox type OR has report_link (these are the only two operations remaining that reference ctx_rec IDs)
