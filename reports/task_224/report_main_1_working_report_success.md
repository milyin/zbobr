# Task 224: Move parameters constants to GitHub backend

## Summary

All three checklist items implemented and committed in a single commit (`54fa338`).

## Changes

### 1. Move PARAM_* constants to github backend (ctx_rec_3)
- Removed 14 `PARAM_*` string constants from `zbobr-api/src/task.rs`
- Removed their re-exports from `zbobr-api/src/lib.rs`
- Added them as `private const` declarations in `zbobr-task-backend-github/src/github.rs`

### 2. Promote pr_url to first-class field in fs backend (ctx_rec_4)
- Replaced `parameters: HashMap<String, String>` with `pr_url: Option<String>` in `TaskFile` struct
- Updated `to_task()` to use `self.pr_url.clone()` directly
- Updated `from_task()` to set `pr_url: task.pr_url.clone()` directly
- Removed `PARAM_PR_URL` import from `zbobr-task-backend-fs/src/fs.rs`

### 3. Decouple dispatcher VAR_* from PARAM_* imports (ctx_rec_5)
- In `zbobr-dispatcher/src/prompts.rs`, replaced aliased constants with inline string literals:
  - `VAR_DESTINATION_REPOSITORY = "destination_repository"`
  - `VAR_DESTINATION_BRANCH = "destination_branch"`
  - `VAR_WORK_BRANCH = "work_branch"`
- Removed `PARAM_DESTINATION_BRANCH`, `PARAM_DESTINATION_REPOSITORY`, `PARAM_WORK_BRANCH` from zbobr_api imports

## Verification
- `cargo build` — clean compilation
- `cargo test` — all tests pass
