Add a new module `params` (or similar) to `zbobr-api` that defines `pub const` string constants for all task parameter keys used across the codebase. Based on current usage in `zbobr-task-backend-github/src/github.rs` and `zbobr-task-backend-fs/src/fs.rs`, the constants needed are:

- `PARAM_DESTINATION_REPOSITORY` = `"destination_repository"`
- `PARAM_DESTINATION_BRANCH` = `"destination_branch"`
- `PARAM_WORK_BRANCH` = `"work_branch"`
- `PARAM_PR_URL` = `"pr_url"`
- `PARAM_STAGE` = `"stage"`
- `PARAM_SIGNAL` = `"signal"`
- `PARAM_STAGE_COUNT` = `"stage_count"`
- `PARAM_FLAG_PAUSE` = `"pause"`
- `PARAM_FLAG_CONFIRM` = `"confirm"`
- `PARAM_FLAG_VALUE_TRUE` = `"true"`

Place these as `pub const` items in a new file (e.g. `zbobr-api/src/params.rs`) and expose them from `zbobr-api/src/lib.rs`. Follow the naming pattern already used for `VAR_*` constants in `prompts.rs` but scoped to the API crate and purpose (task parameter keys).

Why: Centralizing parameter key definitions in the shared API crate ensures all backends use identical strings and prevents future typos or drift between backends.