## Test Implementation Summary

### Checklist item: ctx_rec_54
**Test:** `parse_errors_on_malformed_stage_after_marker` in `zbobr-api/src/context/mod.rs`

**What it verifies:** When `MdContext::from_str` encounters a `<!-- stage -->` marker followed by a `- ...` line with an invalid model token (containing spaces), it returns an error with message containing "Malformed stage title after <!-- stage --> marker" instead of silently skipping the line.

**Commit:** 206ddc85

### Test Results
- All workspace tests pass (257 total, 8 ignored)
- One pre-existing failure in `zbobr` crate (`default_workflow_includes_test_stages`) is unrelated to this change — it existed before the commit