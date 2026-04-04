# Fix: Incorrect Timezone of Interspersed Comments

## Summary
All 6 checklist items completed. The timezone injection pattern was already partially implemented in previous commits. This session completed the final item: updating test construction sites.

## Changes Made

### Already committed (prior sessions):
1. **`zbobr-task-backend-github/src/config.rs`** — Added `pub timezone_offset_seconds: Option<i32>` with `#[config(skip_args)]`
2. **`zbobr-task-backend-fs/src/config.rs`** — Same field added
3. **`zbobr-task-backend-github/src/github.rs`** — Apply `.with_timezone()` in `get_task_comments_internal()` using configured offset
4. **`zbobr-task-backend-fs/src/fs.rs`** — Apply `.with_timezone()` in `read_comments_structured()`
5. **`zbobr/src/commands.rs`** — Inject `timezone_offset_seconds` from dispatcher config at backend construction

### This session (commit d2d8cc72):
6. **`zbobr-dispatcher/tests/mcp_integration/env.rs`** — Added `timezone_offset_seconds: None` to both `ZbobrTaskBackendFsConfig` and `ZbobrTaskBackendGithubConfig` construction sites
7. **`zbobr-task-backend-github/src/github.rs`** (test helper) — Added `timezone_offset_seconds: None` to `make_config()` in unit tests

## Verification
`cargo check --tests` passes cleanly with no errors.