## What

Move all `PARAM_*` string constants (PARAM_DESTINATION_REPOSITORY, PARAM_DESTINATION_BRANCH, PARAM_WORK_BRANCH, PARAM_PR_URL, PARAM_STACK, PARAM_PIPELINE, PARAM_STAGE, PARAM_SIGNAL, PARAM_PIPELINE_RUN_ID, PARAM_STAGE_COUNT, PARAM_MAX_STAGE_COUNT, PARAM_FLAG_PAUSE, PARAM_FLAG_CONFIRM, PARAM_FLAG_VALUE_TRUE) out of the api crate and into the github task backend.

## Why

These constants are the internal representation of `Task` fields in the GitHub issue body format. They are GitHub-backend-specific and do not belong in the public API. Other backends (fs) don't use a named-parameter hashmap at all.

## How to apply

- Remove all `PARAM_*` constants from `zbobr-api/src/task.rs`
- Remove their re-exports from `zbobr-api/src/lib.rs`
- Define them in `zbobr-task-backend-github/src/github.rs` (or a dedicated `params.rs` module in that crate, re-exported from `lib.rs` within the crate only)
- The github backend already imports all of these from zbobr_api — update the import to be local definitions instead
- No backward compatibility needed; these were never intended to be public API