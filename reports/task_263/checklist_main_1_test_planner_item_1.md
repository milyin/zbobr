## Test: MdRecord Display - non-interactive records suppress ctx_rec ID

**File:** `zbobr-api/src/context/mod.rs` (test module)

**What changed:** The `fmt::Display` for `MdRecord` now checks `is_interactive` before emitting `[ctx_rec_N]` in `for_prompt=true` mode. Non-interactive records (non-checkbox types without `report_link`) no longer emit the ID tag.

**Existing coverage gap:** The existing `md_record_display_for_prompt` test (line 1291) only tests an interactive record (Success with report_link). There are no unit tests for the new non-interactive suppression behavior.

**Tests to add:**

1. `md_record_display_for_prompt_non_interactive_suppresses_id` — Construct MdRecord instances for each non-interactive variant (Comment, Success, Failure, Question — all without `report_link`, with `for_prompt: true`). Assert that `to_string()` does NOT contain `ctx_rec_` or `[ctx_rec_`. Assert the output is just the prefix + brief (e.g., `"- 💬 Some comment"`).

2. `md_record_display_for_prompt_checkbox_shows_id` — Construct MdRecord with `CheckboxUnchecked` and `CheckboxChecked` (without `report_link`, `for_prompt: true`). Assert output DOES contain `[ctx_rec_N]`. This verifies checkboxes are always interactive.

3. `md_record_display_for_prompt_with_link_shows_id` — Construct MdRecord for a non-checkbox type (e.g., Comment) WITH `report_link`, `for_prompt: true`. Assert output contains `[ctx_rec_N]` but NOT `<sub>` or the URL. This verifies `report_link` makes any record interactive in prompt mode.

**Pattern:** Follow the existing `md_record_display_for_prompt` test at line 1291 for style/structure.