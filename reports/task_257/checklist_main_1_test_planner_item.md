# Test: Display impls for for_prompt=true rendering

**File:** `zbobr-api/src/context/mod.rs` (in `mod tests`)

**Rationale:** The for_prompt rendering is verified indirectly through the full serialization tests, but individual Display impl behavior for for_prompt=true is not unit-tested. Adding isolated tests catches regressions more precisely.

**Test names:**
1. `md_record_display_for_prompt` — Verify that `MdRecord { for_prompt: true, .. }` renders as `✅ brief [ctx_rec_N]` (no `<sub>`, no URL).
2. `md_compact_comment_display_for_prompt` — Verify that `MdCompactComment { for_prompt: true, .. }` renders as `- user:**name** text` (no timestamp, no URL link).
3. `md_stage_display_for_prompt` — Verify that `MdStage { for_prompt: true, .. }` renders stage name only (not full title with metadata).

**Priority:** LOW — these are already indirectly covered by the integration-level serialization tests, but unit tests add precision.