# Fix Summary for Task #253 Review Issues

## Issue 1 (ctx_rec_18): Unicode panic in sanitize_branch_postfix
**File:** `zbobr-dispatcher/src/cli.rs`
- Changed byte-slice truncation `result[..50]` to char-based: `result.chars().take(50).collect::<String>()`
- Added `sanitize_branch_postfix_unicode_no_panic` test

## Issue 2 (ctx_rec_19): Leftover per-task routing causing inconsistent single-repo behavior
Removed `destination_repository` and `destination_branch` from `Task` struct and all usages:

- **`zbobr-api/src/task.rs`**: Removed fields from `Task` struct
- **`zbobr-api/src/backend.rs`**: Removed `set_destination_repository`/`set_destination_branch` default impls from `TaskBackend` trait
- **`zbobr-dispatcher/src/task.rs`**: Removed getter/setter methods; fixed `finish()` to use `repo_backend.repo_name()` instead of per-task field
- **`zbobr-dispatcher/src/prompts.rs`**: Removed reading of per-task fields from `build_template_variables`; `VAR_DESTINATION_REPOSITORY`/`VAR_DESTINATION_BRANCH` now injected via `extra_vars` from repo_backend
- **`zbobr-dispatcher/src/cli.rs`**: Removed writes of dest fields in `ensure_work_branch`
- **`zbobr-dispatcher/src/lib.rs`**: Re-exported `VAR_DESTINATION_BRANCH`/`VAR_DESTINATION_REPOSITORY`
- **`zbobr-dispatcher/src/workflow.rs`**: Fixed test Task construction
- **`zbobr-task-backend-github/src/github.rs`**: Removed PARAM constants and read/write of these fields
- **`zbobr-task-backend-fs/src/fs.rs`**: Removed from `TaskFile` struct
- **`zbobr/src/commands.rs`**: Inject from `repo_backend` into `ConfiguredPromptBuilder.with_var()`; added `WorktreeBackend` trait import; removed from `TaskSubcommand::Update` match arm
- **`zbobr-dispatcher/tests/mcp_integration/env.rs`**: Simplified `update_task_branches` signature
- **`zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs`**: Updated all 12 callers

## Issue 3 (ctx_rec_20): Incorrect preparator-removal test
**File:** `zbobr/src/init.rs`
- Changed `contains_key("preparator")` → `contains_key("preparing")` (the actual old stage key)
- Changed `workflow.roles.keys()` check to iterate `main.stages.values()` checking `stage_def.role` field for `"preparator"`

All tests pass (excluding pre-existing `compact_comment_uses_first_line_only` failure in zbobr-api).