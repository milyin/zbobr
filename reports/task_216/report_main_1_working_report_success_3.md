# Parameter Name Constants Refactor

## Summary

All parameter name string literals have been replaced with named constants, eliminating duplication and making the parameter names single-source-of-truth.

## Changes

### zbobr-api/src/task.rs
Added 14 new public constants:
- `PARAM_DESTINATION_REPOSITORY`, `PARAM_DESTINATION_BRANCH`, `PARAM_WORK_BRANCH`, `PARAM_PR_URL`
- `PARAM_STACK`, `PARAM_PIPELINE`, `PARAM_STAGE`, `PARAM_SIGNAL`
- `PARAM_PIPELINE_RUN_ID`, `PARAM_STAGE_COUNT`, `PARAM_MAX_STAGE_COUNT`
- `PARAM_FLAG_PAUSE`, `PARAM_FLAG_CONFIRM`, `PARAM_FLAG_VALUE_TRUE`

### zbobr-api/src/lib.rs
Exported all new PARAM_* constants from the crate root.

### zbobr-task-backend-github/src/github.rs
- Removed local `FLAG_PAUSE`, `FLAG_CONFIRM`, `FLAG_VALUE_TRUE` constants (now from zbobr-api)
- Replaced all 15+ parameter name string literals in `issue_to_task` and `task_to_string_params` with PARAM_* constants
- Updated test module references to use the new constants (via `use super::*`)

### zbobr-task-backend-fs/src/fs.rs
- Replaced `"pr_url"` literals with `PARAM_PR_URL` in `to_task` and `from_task`

### zbobr-dispatcher/src/prompts.rs
- Updated `VAR_DESTINATION_REPOSITORY`, `VAR_DESTINATION_BRANCH`, `VAR_WORK_BRANCH` to reference the corresponding `PARAM_*` constants from zbobr-api (eliminating duplicate string values)

## Test Results
All 127 tests pass (95 test cases + 9 ignored integration tests).
