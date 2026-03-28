# Plan: Systematically use constants for all task parameter names

## Problem
Parameter name strings like `"destination_repository"`, `"pr_url"`, `"stage"`, `"signal"`, etc. are used as bare string literals in `zbobr-task-backend-github/src/github.rs` and `zbobr-task-backend-fs/src/fs.rs`. The flag names `FLAG_PAUSE`/`FLAG_CONFIRM` already have module-local constants in `github.rs`, but are not shared. The dispatcher's `prompts.rs` has `VAR_*` constants for 3 of the same strings but defined independently.

## Approach
1. **Add `params` module to `zbobr-api`** — define all task parameter key constants (`PARAM_DESTINATION_REPOSITORY`, `PARAM_DESTINATION_BRANCH`, `PARAM_WORK_BRANCH`, `PARAM_PR_URL`, `PARAM_STAGE`, `PARAM_SIGNAL`, `PARAM_STAGE_COUNT`, `PARAM_FLAG_PAUSE`, `PARAM_FLAG_CONFIRM`, `PARAM_FLAG_VALUE_TRUE`). Export from `zbobr-api/src/lib.rs`.

2. **Replace literals in github backend** — use the new shared constants everywhere in `github.rs`; remove local `FLAG_*` constant declarations.

3. **Replace literals in fs backend** — use `PARAM_PR_URL` in `fs.rs`.

4. **Update dispatcher prompts** — redefine `VAR_DESTINATION_*` / `VAR_WORK_BRANCH` constants as aliases of the shared API constants, eliminating string duplication.

## Analog
The existing `VAR_*` constants in `prompts.rs` serve as the pattern to follow for naming and placement, just moved to `zbobr-api` and renamed with `PARAM_` prefix for clarity.

## Key constraint
All parameter names must remain unchanged as strings — only the usage at call sites changes from literals to named constants.