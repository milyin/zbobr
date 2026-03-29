# Task 239 — Implementation Complete

## Summary

All 6 checklist items are complete. The previous session handled items ctx_rec_2 through ctx_rec_5 (instance config field, GitHub backend setup/filtering). This session completed ctx_rec_6 and ctx_rec_7.

## Changes Made

### ctx_rec_6: Add `instance` to `StageInfo` and update `MdStageTitle` format

**`zbobr-api/src/task.rs`**
- Added `instance: String` field as the first field of `StageInfo`

**`zbobr-api/src/context/stage_title.rs`**
- Added `instance: String` field to `MdStageTitle`
- Updated `PipelineStage` wrapper to include `instance` and format as `instance:pipeline:run_id:**stage**`
- Updated `Display for MdStageTitle` and `Display for MdMdStageTitleForPrompt` to pass instance
- Updated `From<&StageInfo>` and `From<MdStageTitle>` conversions
- Updated `parse_next_pipeline_stage` to parse `instance:pipeline:run_id` prefix
- Updated all tests with `instance` field and new format strings

### ctx_rec_7: Populate `instance` when constructing `StageInfo` in dispatcher

**`zbobr-dispatcher/src/cli.rs`**
- When pushing a new `StageContext`, capture `instance` from `self.zbobr.config().instance` and include it in `StageInfo`

### Additional fixes

**`zbobr-task-backend-github/src/github.rs`**
- Added missing `IssueUser` struct (pre-existing compilation error from commit 43806f1)

**`zbobr-task-backend-github/src/separator.rs`**, **`zbobr-dispatcher/src/task.rs`**, **`zbobr-dispatcher/tests/mcp_integration/env.rs`**, **`zbobr-api/src/context/mod.rs`**
- Updated all test `StageInfo` constructions to include `instance: "default".to_string()`
- Updated test string assertions to include instance prefix in stage title format
- Fixed `roundtrip_preserves_context` to use ID-based record lookup (records are reordered on serialization)

## Test Results

All tests pass:
- zbobr-api: 45 passed
- zbobr-dispatcher: 39 passed  
- zbobr-task-backend-github: 9 passed (+ 9 integration tests ignored)
