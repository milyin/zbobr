# Test: Empty stages filtered out in for_prompt mode

**File:** `zbobr-api/src/context/mod.rs` (in `mod tests`)

**Rationale:** The user explicitly requested "when stage contains no subitems, filter it out." Code at line ~603 implements `if for_prompt && md_stage.records.is_empty() { continue; }` but NO test covers this behavior.

**Test name:** `for_prompt_filters_empty_stages`

**Setup:**
- Create a `TaskContext` with 2+ stages where at least one stage has records and at least one has zero records (empty).

**Assertions:**
- When `serialize_context(&ctx, &[], true, None)` is called (for_prompt=true):
  - The output should contain the stage name for stages WITH records
  - The output should NOT contain the stage name for stages WITHOUT records
- When `serialize_context(&ctx, &[], false, None)` is called (for_prompt=false):
  - ALL stages should appear (empty stages not filtered)

**Priority:** HIGH — this is a user-requested feature with zero test coverage.