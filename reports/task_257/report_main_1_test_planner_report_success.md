# Test Plan: Remove Noise from Context for Prompt

## Summary

The implementation introduces two features:
1. **Simplified for-prompt context rendering** — stage name only, plain `[ctx_rec_N]`, no timestamps/links, empty stage filtering
2. **`get_ctx_rec` MCP tool** — returns report file content or brief for a context record

Existing tests were updated to match the new rendering format. However, several gaps remain.

## Existing Coverage (already passing)

| Area | Tests | Status |
|------|-------|--------|
| Stage name-only headers (for_prompt) | `serialize_for_prompt_omits_prompt_link` | ✅ |
| No metadata in for_prompt mode | Same test | ✅ |
| Plain `[ctx_rec_N]` format | Same test | ✅ |
| Comment without timestamp/link | `for_prompt_true_uses_compact_comment_format` | ✅ |
| No truncation in for_prompt | `for_prompt_true_does_not_truncate_long_comment_text` | ✅ |
| Tool filtering includes get_ctx_rec | `all_tool_names_match_router`, `filtering_works` | ✅ |

## Test Gaps (5 groups)

### HIGH Priority

1. **Empty stage filtering** (`context/mod.rs`)
   - User explicitly requested this. Code exists but no test verifies it.
   - Must test that stages with zero records are excluded in for_prompt=true but included in for_prompt=false.

2. **`get_context_record_content` method** (`dispatcher/task.rs`)
   - New method with 3 branches: report link → file content, no link → brief, not found → None.
   - Use existing TrackingBackend test infrastructure.

### MEDIUM Priority

3. **`get_ctx_rec` MCP tool integration** (`dispatcher/task.rs`)
   - End-to-end test through UnifiedMcp using `make_test_mcp()`.
   - Validates wiring: tool invocation → param parsing → session method → response.

4. **`parse_ctx_rec_id` unit tests** (`mcp/common.rs`)
   - Shared utility with zero coverage. Test numeric, prefixed, and error cases.

### LOW Priority

5. **Display unit tests for for_prompt=true** (`context/mod.rs`)
   - Isolated tests for MdRecord, MdCompactComment, MdStage Display when for_prompt=true.
   - Already covered indirectly by serialization tests; these add precision.

## Pre-existing Issue

One test (`compact_comment_uses_first_line_only`) was already failing before these changes and is unrelated to this feature. It should not block the new test work.