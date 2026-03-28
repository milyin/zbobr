# Plan: Move Parameter Constants to GitHub Backend

## Summary

The `PARAM_*` string constants are currently exported from `zbobr-api` but they represent GitHub issue body encoding details — not a general API contract. The fs backend uses only one of them (`PARAM_PR_URL`) as a workaround via a HashMap. The dispatcher re-exports three of them as prompt template variable names.

## Changes

### 1. Move `PARAM_*` constants from `zbobr-api` to `zbobr-task-backend-github`

- Remove all `PARAM_*` constants from `zbobr-api/src/task.rs`
- Remove them from `zbobr-api/src/lib.rs` re-exports
- Define them locally in `zbobr-task-backend-github/src/github.rs` (or a new `params.rs` submodule)
- No behavioral change — github backend continues to use them internally

### 2. Promote `pr_url` to a first-class field in the fs backend

- `TaskFile` in `zbobr-task-backend-fs/src/fs.rs` still has `parameters: HashMap<String, String>` and stores `pr_url` via `PARAM_PR_URL`
- All other task fields (`destination_repository`, `destination_branch`, `work_branch`, `pause`, `confirm`, etc.) are already first-class YAML fields
- Add `pr_url: Option<String>` directly to `TaskFile` struct
- Remove the `parameters: HashMap<String, String>` field entirely
- Update `to_task()` to read `self.pr_url` directly
- Update `from_task()` to write `pr_url` directly without going through a HashMap
- Drop the `PARAM_PR_URL` import from `zbobr-api`

### 3. Decouple dispatcher `VAR_*` constants from `PARAM_*`

- In `zbobr-dispatcher/src/prompts.rs`, `VAR_DESTINATION_REPOSITORY`, `VAR_DESTINATION_BRANCH`, and `VAR_WORK_BRANCH` are currently aliases for the corresponding `PARAM_*` values
- Replace them with standalone string literals (`"destination_repository"`, `"destination_branch"`, `"work_branch"`)
- Drop the `PARAM_DESTINATION_BRANCH`, `PARAM_DESTINATION_REPOSITORY`, `PARAM_WORK_BRANCH` imports from `zbobr-api`

## No backward compatibility needed

The task explicitly states the fs backend is for testing only and the system is not deployed. Old YAML files with a `parameters.pr_url` entry will simply lose that field on next write — acceptable.
