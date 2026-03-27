## Changes

### 1. Fix token leakage in cleanup_legacy_token_config() error logging
**File:** `zbobr-repo-backend-github/src/github.rs`

Replaced the `git()` helper call with direct `tokio::process::Command` invocation. The `git()` helper embeds the full args (including the legacy key containing the token) into its error message via `anyhow::bail!`. Now:
- stdout/stderr are suppressed (Stdio::null)
- Only the redacted key and exit code are logged
- No error object that could contain the raw token is ever formatted

### 2. Add fetch_refs() trait method and replace update_worktree() in overwrite_author()
**Files:** `zbobr-api/src/backend.rs`, `zbobr-repo-backend-github/src/github.rs`, `zbobr-repo-backend-fs/src/fs.rs`, `zbobr-dispatcher/src/lib.rs`, `zbobr-dispatcher/src/task.rs`, `zbobr/src/commands.rs`

- Added `fetch_refs(&self, identity: &TaskIdentity) -> anyhow::Result<()>` to `WorktreeBackend` trait
- GitHub backend: calls `ensure_bare_clone_github` + `git_env(fetch origin)` with token auth env — fetch only, no merges/pushes/PR creation
- FS backend: calls `ensure_bare_clone` + `git(fetch origin)` — fetch only
- DummyRepo (test): no-op returning Ok
- ZbobrDispatcher: added `fetch_refs()` wrapper delegating to backend
- overwrite_author(): replaced `zbobr.update_worktree(&identity).await?` with `zbobr.fetch_refs(&identity).await?`, restoring side-effect-free behavior for dry-run mode

### Build & Test
All 112 tests pass. No warnings in production code.