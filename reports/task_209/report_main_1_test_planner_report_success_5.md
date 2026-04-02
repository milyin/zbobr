## Test Plan: Stage-Title Error Propagation in MdContext

### Changes Analyzed

Commit `cde5c657` added `after_stage_marker` tracking to `MdContext::from_str` so that when a `<!-- stage -->` marker precedes a `- ...` line, the parse error from `MdStageTitle::from_str` is propagated instead of silently ignored.

### Coverage Assessment

| Area | Status |
|------|--------|
| `detect_quota_failure()` (7 tests) | ✅ Already covered (commit a7893518) |
| `MdStageTitle::from_str` malformed model rejection (2 tests) | ✅ Already covered (commit a7893518) |
| `ExecutorOutput.quota_failure` integration in executors | ✅ Low-value unit test (simple boolean pass-through) |
| `cli.rs` quota_failure → connectivity_failure mapping | ✅ Integration-level, covered by existing flow |
| Valid stage title after `<!-- stage -->` marker | ✅ Covered by `compact_comment_roundtrip_preserves_context` |
| **Malformed stage title after `<!-- stage -->` marker → error** | ❌ **Not tested** |

### Tests Required

**1 test** (1 checklist item):

- `parse_errors_on_malformed_stage_after_marker` in `zbobr-api/src/context/mod.rs` — verifies that `MdContext::from_str` returns an error containing "Malformed stage title after <!-- stage --> marker" when a `<!-- stage -->` marker is followed by a `- ...` line with an invalid model token.