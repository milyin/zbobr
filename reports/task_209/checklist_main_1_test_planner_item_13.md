## Test: MdContext::from_str rejects malformed stage title after `<!-- stage -->` marker

**File:** `zbobr-api/src/context/mod.rs` (in the existing `#[cfg(test)] mod tests` block)

**Test name:** `parse_errors_on_malformed_stage_after_marker`

**What it covers:** The `after_stage_marker` tracking added in commit `cde5c657`. When `MdContext::from_str` encounters a `<!-- stage -->` marker followed by a `- ...` line that fails `MdStageTitle::from_str` parsing (e.g. a model token containing spaces), it should return an error instead of silently skipping the line.

**Setup:**
Construct a raw context string with:
1. A valid stage title line (so there's at least one valid stage to show the parser works normally)
2. A `<!-- stage -->` marker
3. A `- ...` line with an invalid model token (e.g. `bad model` with a space)

**Assertions:**
- `parse_context(...)` returns `Err`
- The error message contains `"Malformed stage title after <!-- stage --> marker"`

**Why this test matters:**
- The review in ctx_rec_51 identified that `MdContext::from_str` was silently dropping malformed stage titles even after a `<!-- stage -->` marker, defeating the purpose of the lower-level `MdStageTitle::from_str` rejection of invalid model tokens.
- Commit `cde5c657` fixed this, but no test was added for the fix.
- Without this test, a future refactor could regress back to silent acceptance.

**Existing coverage notes:**
- The happy path (valid stage title after marker) is already covered by `compact_comment_roundtrip_preserves_context` (line 1298).
- The lower-level `MdStageTitle::from_str` rejection of bad model tokens is covered by `parse_rejects_malformed_model_token` in `stage_title.rs`.
- This test bridges the gap: it verifies the error propagates through the higher-level `MdContext::from_str` parser.