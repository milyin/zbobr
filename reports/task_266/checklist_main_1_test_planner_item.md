## Test: `validate_all_prompts_aggregates_multiple_errors`

**File:** `zbobr-dispatcher/src/prompts.rs` (test module)

**Purpose:** Verify that `validate_all_prompts()` collects and reports errors from ALL failing stages, rather than stopping at the first error. This tests the error aggregation loop at lines 93-105.

**Setup:**
- Create a workflow with 2+ stages that will both fail (e.g., two stages referencing different nonexistent prompt files, or one with a missing file and another with an undefined variable)
- Use the existing `make_prompt_builder()` test helper

**Assertions:**
- `validate_all_prompts()` returns `Err`
- The error message contains references to BOTH failing stages (check that both stage names appear in the error string)
- This confirms the function doesn't short-circuit on first error

**Pattern:** Follow existing test style — use `result.unwrap_err().to_string()` and `assert!(err.contains(...))` checks.