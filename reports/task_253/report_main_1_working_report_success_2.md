# Fix: zbobr crate compilation errors

## Problem
The zbobr crate failed to compile with two errors in `zbobr/src/commands.rs`:

1. `unresolved imports VAR_DESTINATION_BRANCH, VAR_DESTINATION_REPOSITORY` — these constants exist in `zbobr-dispatcher/src/prompts.rs` but were not re-exported from the dispatcher's `lib.rs`.

2. `missing fields destination_branch and destination_repository` — the `Task` struct in `zbobr-api` still has these fields, but the `dummy_task_and_comments()` function in `commands.rs` didn't initialize them.

## Fix

### `zbobr-dispatcher/src/lib.rs`
Added `VAR_DESTINATION_BRANCH` and `VAR_DESTINATION_REPOSITORY` to the `pub use prompts::` re-export.

### `zbobr/src/commands.rs`
Added `destination_branch: None` and `destination_repository: None` to the `Task` struct initializer in `dummy_task_and_comments()`.

## Result
- `cargo build` succeeds with no errors
- 46 tests pass; 1 pre-existing unrelated failure (`compact_comment_uses_first_line_only`) remains, as noted in ctx_rec_22
- Committed as `73ad21d`