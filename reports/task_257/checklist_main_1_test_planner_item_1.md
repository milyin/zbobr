# Test: get_context_record_content method on RoleSession

**File:** `zbobr-dispatcher/src/task.rs` (in `mod comment_model_tests`)

**Rationale:** `get_context_record_content` is a new method with three code paths (report link present → read file, no link → return brief, record not found → return None). None are tested.

**Test name:** `get_context_record_content_returns_report_or_brief`

**Setup:** Use existing `make_test_parts()` / `TrackingBackend` infrastructure. Create a task with:
- A context record that HAS a `report_link` (store a report file via the backend)
- A context record that has NO `report_link` (brief only)

**Assertions:**
1. Calling `session.get_context_record_content(id_with_link)` returns `Ok(Some(report_file_content))`
2. Calling `session.get_context_record_content(id_without_link)` returns `Ok(Some(brief_text))`
3. Calling `session.get_context_record_content(nonexistent_id)` returns `Ok(None)`

**Priority:** HIGH — new logic with branching that should be verified.