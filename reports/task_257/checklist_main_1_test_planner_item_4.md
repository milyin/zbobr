# Test: parse_ctx_rec_id unit tests

**File:** `zbobr-dispatcher/src/mcp/common.rs` (in `mod tests`)

**Rationale:** `parse_ctx_rec_id` is a shared parsing utility used by both `delete_ctx_rec` and the new `get_ctx_rec`. It has zero test coverage despite handling three distinct cases.

**Test name:** `parse_ctx_rec_id_variants`

**Assertions:**
1. `parse_ctx_rec_id("5")` returns `Ok(5)` — bare numeric
2. `parse_ctx_rec_id("ctx_rec_5")` returns `Ok(5)` — prefixed format
3. `parse_ctx_rec_id("abc")` returns `Err(...)` — invalid string
4. `parse_ctx_rec_id("ctx_rec_abc")` returns `Err(...)` — invalid suffix
5. `parse_ctx_rec_id("")` returns `Err(...)` — empty string

**Priority:** MEDIUM — small function but used by multiple tools; easy to add.