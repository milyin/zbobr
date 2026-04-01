## Test Plan: Remove DeleteCtxRec Action

### Summary

The implementation removed the `DeleteCtxRec` MCP tool and introduced new behavior in `MdRecord::fmt` that suppresses `[ctx_rec_N]` ID tags for non-interactive records in prompt mode. The deletion-related changes are well-handled (removed code, removed tests). The **new behavior** (ID suppression for non-interactive records) has test coverage gaps.

### Analysis of Changes

1. **Removed code** (DeleteCtxRec enum variant, handler, param struct, integration test, role configs) — No new tests needed; existing tests that referenced this tool were correctly removed.

2. **New behavior** (`MdRecord::fmt` — `is_interactive` check) — This is the area needing test coverage. A record is "interactive" if it's a checkbox OR has a `report_link`. Non-interactive records in `for_prompt=true` mode now omit the `[ctx_rec_N]` tag entirely.

### Coverage Gaps Identified

| Gap | Location | Issue |
|-----|----------|-------|
| No unit test for non-interactive suppression | `md_record_display_for_prompt` (line 1291) | Only tests interactive record (Success+link) |
| No checkbox unit test in prompt mode | Missing | Checkboxes without link not tested |
| Missing negative assertions | `serialize_for_prompt_omits_prompt_link` (line 798) | Has non-interactive records (id=5,6) but doesn't assert IDs are absent |
| Missing negative assertion | `md_stage_display_for_prompt` (line 1325) | Has non-interactive record but doesn't check ID absence |
| No mixed-record e2e test | `for_prompt_renders_complete_format` (line 1453) | All records are interactive |

### Test Plan (3 checklist items)

1. **Unit tests for MdRecord non-interactive ID suppression** — New test functions covering all non-interactive variants (Comment/Success/Failure/Question without link), checkbox variants (always interactive), and link-makes-interactive behavior.

2. **Strengthen existing prompt-mode tests** — Add negative assertions to `serialize_for_prompt_omits_prompt_link`, `md_stage_display_for_prompt`, and `for_prompt_filters_empty_stages` to verify non-interactive record IDs are absent.

3. **End-to-end mixed interactive/non-interactive test** — New test with a context containing both interactive and non-interactive records, verifying IDs appear only for interactive ones.

All tests go in `zbobr-api/src/context/mod.rs` test module, following existing patterns.