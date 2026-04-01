# Test Plan — Round 3 (post e1bb556 fix)

## Summary

Two additional tests recommended. The latest working stage (commit e1bb556) fixed two review-blocking issues:
1. Stage markers now gated to non-prompt mode
2. Non-prompt comment format restored to `user:**name**` bold style

Existing test coverage from rounds 1-2 is comprehensive at the component level (17 new tests across 3 files). However, two gaps remain:

## Recommended Tests

### 1. End-to-end prompt format validation (**High Priority**)

Individual components are well-tested, but there is no test that validates the **complete composed output** of a realistic prompt context against the task requirements. This is the most important gap because review rounds 1 and 2 both caught composition-level issues (stage markers leaking, format regression) that component tests missed.

- Build a realistic context with multiple stages (one empty), records of various types, and interleaved comments
- Validate complete output matches the specified format: stage-name-only headers, plain `[ctx_rec_N]` tags, plain user comments, empty stages filtered, no markers

### 2. Multi-line comment in for_prompt mode (**Medium Priority**)

All existing for_prompt comment tests use single-line bodies. The for_prompt path intentionally preserves full multi-line bodies, but this isn't tested. Important because the non-prompt path now extracts first-line-only, creating an easy regression risk.

## Already Well-Tested (no additional tests needed)

| Area | Tests | Status |
|------|-------|--------|
| MdRecord Display for_prompt | `md_record_display_for_prompt` | ✅ |
| MdCompactComment Display for_prompt | `md_compact_comment_display_for_prompt` | ✅ |
| MdStage Display for_prompt | `md_stage_display_for_prompt` | ✅ |
| Empty stage filtering | `for_prompt_filters_empty_stages` | ✅ |
| Stage marker suppression | `stage_marker_not_added_in_prompt_mode` | ✅ |
| parse_ctx_rec_id | 5 unit tests | ✅ |
| get_context_record_content | `get_context_record_content_returns_report_or_brief` | ✅ |
| get_ctx_rec MCP tool | `get_ctx_rec_returns_content` (4 cases) | ✅ |
| MCP integration | abstract_scenarios step | ✅ |
| Non-prompt format preserved | `compact_comment_*` tests | ✅ |
| GetCtxRec config/wiring | Enum variant, ALL_TOOLS, init.rs roles | ✅ |

## Note on pre-existing issue

The test `compact_comment_uses_first_line_only` on origin/main appears to have been broken by commit c47c70e ("single line comment in context") which changed the non-prompt behavior from first-line-only to join-all-lines, without updating the test assertion. This branch's change to use first-line-only for non-prompt actually aligns with the original test intent from commit 4d94f6a. This is a pre-existing issue unrelated to the current task.
