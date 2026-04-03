## Test: TESTER_PROMPT excludes formatting/linting and defers to separate stage

**File:** `zbobr/src/init.rs` (in `mod tests`)

**What to verify:**
1. `TESTER_PROMPT` does NOT contain "Run formatting/linting checks" (removed instruction)
2. `TESTER_PROMPT` does NOT contain "Fix formatting/linting issues" or "fix formatting" as an instruction to the tester (removed step 4)
3. `TESTER_PROMPT` DOES contain text indicating linting is handled by a separate stage (e.g., "separate stage")

**Why:** The core behavioral change of this task is removing formatting/linting responsibility from the tester. This test prevents accidental regression where someone re-adds formatting instructions to the tester prompt.

**Pattern:** Similar to `default_prompts_reference_get_ctx_rec_when_role_has_access` — content assertions on prompt constants.