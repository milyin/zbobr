# Test: get_ctx_rec MCP tool integration

**File:** `zbobr-dispatcher/src/task.rs` (in `mod comment_model_tests`)

**Rationale:** The `get_ctx_rec` MCP tool is a new user-facing feature. An end-to-end test using the existing `make_test_mcp()` infrastructure ensures the full call chain works: tool invocation → parameter parsing → session method → response.

**Test name:** `get_ctx_rec_returns_content`

**Setup:** Use `make_test_mcp()` to create a `UnifiedMcp` instance. Store a report and create a context record pointing to it.

**Assertions:**
1. Calling the `get_ctx_rec` tool with a valid record ID returns the report content
2. Calling with a non-existent ID returns an error message containing "not found"
3. Calling with an invalid ID format (e.g., "abc") returns a parsing error

**Pattern:** Follow the existing `report_success_stores_context_records` test structure.

**Priority:** MEDIUM — the underlying method test (above) covers core logic, but an integration test ensures wiring is correct.