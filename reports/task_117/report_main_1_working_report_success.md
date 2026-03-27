## Summary

Implemented env-based GitHub token auth to prevent token leakage into logs, command args, and on-disk git config.

## Changes (3 files, 1 commit)

### zbobr-utility/src/lib.rs
- Added `git_env()` and `git_check_env()` functions that accept extra environment variables
- Refactored existing `git()` and `git_check()` to delegate to the new `_env` variants with empty envs

### zbobr-repo-backend-github/src/github.rs
- Added `token_auth_env()` method: builds `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/`GIT_CONFIG_VALUE_0` env vars for process-scoped `insteadOf` URL rewrite
- Replaced `configure_token_auth()` with `cleanup_legacy_token_config()` (removes stale on-disk insteadOf entries from existing repos)
- Updated `ensure_bare_clone_github()`: clones with clean URL + env auth, no post-clone URL normalization needed
- Updated `ensure_fork_remote()`, `sync_fork_base_with_upstream()`: fetch fork with env auth
- Updated `fetch_remote_work_branch()`, `push_worktree_to_remote()`: added `envs` parameter
- Updated `update_worktree()`: computes auth env once, passes to all remote-contacting calls

### zbobr/src/commands.rs
- Removed standalone `git fetch origin` call (update_worktree handles fetching internally)
- Removed unused `git` import

## Verification
- `cargo build` succeeds
- `cargo test` passes all 112 tests (8 GitHub integration tests ignored as expected)
- Only remaining `x-access-token` occurrence is inside the env var value in `token_auth_env()`, which is expected