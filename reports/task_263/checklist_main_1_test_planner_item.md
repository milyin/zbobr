## Test: Strengthen existing tests with negative assertions for non-interactive records

**File:** `zbobr-api/src/context/mod.rs` (test module)

**Existing coverage gap:** Several existing tests contain non-interactive records in prompt mode but don't verify that their ctx_rec IDs are absent. The new suppression behavior should be validated in these tests.

**Tests to modify:**

1. **`serialize_for_prompt_omits_prompt_link`** (line 798) — Uses `sample_context()` which contains non-interactive records: id=5 (Comment "Retrying with fix", no link) and id=6 (Question "Should we use async?", no link). Add assertions:
   - `assert!(!output.contains("[ctx_rec_5]"))` — non-interactive Comment should not have ID
   - `assert!(!output.contains("[ctx_rec_6]"))` — non-interactive Question should not have ID
   - Also verify interactive records ARE present: `assert!(output.contains("[ctx_rec_1]"))` (checkbox), `assert!(output.contains("[ctx_rec_2]"))` (checkbox), `assert!(output.contains("[ctx_rec_4]"))` (Failure with link)

2. **`md_stage_display_for_prompt`** (line 1325) — Contains a non-interactive record (Success, id=1, no report_link, for_prompt=true). Add assertion:
   - `assert!(!rendered.contains("ctx_rec_1"))` — non-interactive record should not emit any ctx_rec tag

3. **`for_prompt_filters_empty_stages`** (line 1360) — Contains non-interactive records: id=1 (Success, no link) and id=2 (Comment, no link). Add assertions to the `prompt_output`:
   - `assert!(!prompt_output.contains("[ctx_rec_1]"))` — non-interactive Success
   - `assert!(!prompt_output.contains("[ctx_rec_2]"))` — non-interactive Comment

**Pattern:** Add the negative assertions near existing related assertions in each test function.