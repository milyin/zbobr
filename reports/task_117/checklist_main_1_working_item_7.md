## Problem
In `overwrite_author()` (commands.rs:701), `zbobr.update_worktree(&identity).await?` was used to replace the original `git fetch origin dest_branch`. But `update_worktree()` does merges, pushes, PR creation, etc. — heavy side effects that violate dry-run expectations.

## Fix
Add a `fetch_refs` method to `WorktreeBackend` trait that does auth-safe fetch without any other side effects. Then add a wrapper in `ZbobrDispatcher` and use it in `overwrite_author()` instead of `update_worktree()`.

The `fetch_refs` method should:
- Accept identity and workspace_path
- In the GitHub backend: do `git_env(bare_dir, &["fetch", "origin"], &env)` with token_auth_env
- In the FS backend: be a no-op (return Ok)

Then in `overwrite_author`, call `zbobr.fetch_refs(&identity).await?` before the dry-run/rewrite logic.