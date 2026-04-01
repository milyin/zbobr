## Test: End-to-end prompt format with mixed interactive/non-interactive records

**File:** `zbobr-api/src/context/mod.rs` (test module)

**Existing coverage gap:** The `for_prompt_renders_complete_format` test (line 1453) only uses interactive records (checkboxes and records with report_link). There is no end-to-end test that verifies the full prompt output when BOTH interactive and non-interactive records coexist in the same context.

**Test to add:**

`for_prompt_mixed_interactive_and_non_interactive` — Build a TaskContext with a single stage containing a mix of:
- Checkbox unchecked (interactive, no link) — e.g., id=1 "Implement feature"
- Comment without link (non-interactive) — e.g., id=2 "Starting work"
- Success with report_link (interactive) — e.g., id=3 "Phase 1 done"
- Failure without link (non-interactive) — e.g., id=4 "Build failed"
- Question without link (non-interactive) — e.g., id=5 "Need clarification"

Serialize with `for_prompt=true` and assert:
- `[ctx_rec_1]` present (checkbox is always interactive)
- `[ctx_rec_2]` absent (Comment without link)
- `[ctx_rec_3]` present (has report_link)
- `[ctx_rec_4]` absent (Failure without link)
- `[ctx_rec_5]` absent (Question without link)
- No `<sub>` tags anywhere
- No report URLs anywhere
- Records are properly indented

This provides a single comprehensive test that validates the complete interaction between interactive/non-interactive classification and prompt-mode serialization.

**Pattern:** Follow the structure of `for_prompt_renders_complete_format` at line 1453.