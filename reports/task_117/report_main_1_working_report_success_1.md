# Fix: Address Review Feedback on Token Security

## Changes Made

### 1. Replace URL-based auth with http.extraheader (`github.rs:token_auth_env()`)
- **Before**: Used `url.https://x-access-token:{token}@github.com/.insteadOf` which still embedded the token in a URL string. Git errors/logs could expose it.
- **After**: Uses `http.https://github.com/.extraheader` with `Authorization: basic <base64("x-access-token:" + token)>`. The token is now only passed as an HTTP header, never in any URL.
- Added `base64 = "0.22"` dependency to `zbobr-repo-backend-github/Cargo.toml`.

### 2. Restore auth-safe fetch in overwrite_author (`commands.rs`)
- **Before**: The fetch `git(&repo_dir, &["fetch", "origin", dest_branch])` was removed without replacement, leaving `dest_branch` potentially stale.
- **After**: Calls `zbobr.update_worktree(&identity)` before the rewrite, which performs a full auth-aware fetch+merge cycle through the backend. This ensures `dest_branch` ref is current for both dry-run log output and the actual `filter-branch` range computation.

### 3. Improve error handling in cleanup_legacy_token_config (`github.rs`)
- **Before**: Returned nothing (`async fn ... -> ()`), silently ignoring all failures.
- **After**: Returns `anyhow::Result<()>`, logs warnings via `tracing::warn!` for individual `git config --unset` failures, and propagates the result to the caller.

## Files Modified
- `zbobr-repo-backend-github/Cargo.toml` — added `base64 = "0.22"` dependency
- `zbobr-repo-backend-github/src/github.rs` — token_auth_env() and cleanup_legacy_token_config()
- `zbobr/src/commands.rs` — restored auth-safe fetch in overwrite_author

## Verification
- `cargo build`: success
- `cargo test`: all 112 tests pass (8 GitHub integration tests appropriately ignored)