In `zbobr-task-backend-github/src/github.rs`, replace all parameter name string literals with the new constants from `zbobr-api`. Specifically:

- Replace `"destination_repository"` → `PARAM_DESTINATION_REPOSITORY`
- Replace `"destination_branch"` → `PARAM_DESTINATION_BRANCH`
- Replace `"work_branch"` → `PARAM_WORK_BRANCH`
- Replace `"pr_url"` → `PARAM_PR_URL`
- Replace `"stage"` → `PARAM_STAGE`
- Replace `"signal"` → `PARAM_SIGNAL`
- Replace `"stage_count"` → `PARAM_STAGE_COUNT`

The local module constants `FLAG_PAUSE`, `FLAG_CONFIRM`, and `FLAG_VALUE_TRUE` should be removed and replaced with the new shared constants `PARAM_FLAG_PAUSE`, `PARAM_FLAG_CONFIRM`, and `PARAM_FLAG_VALUE_TRUE` from zbobr-api.

Import the new constants from `zbobr_api` and remove the now-redundant local `const` declarations.

Why: Eliminates all bare string literals for parameter names in the GitHub backend, enforcing the constant-over-literal pattern consistently.