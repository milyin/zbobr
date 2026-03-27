## File: `zbobr/src/commands.rs`

### Problem
Line 699: `git(&repo_dir, &["fetch", "origin", dest_branch]).await?;`

This standalone `git fetch` runs in a worktree directory. Previously it worked because the bare repo had `insteadOf` auth in its persistent git config (inherited by worktrees). With the env-based approach, the persistent config is removed, so this fetch will fail with authentication errors.

### Solution
Remove the standalone fetch at line 699. The `update_worktree` call at line 711 already fetches origin internally (Phase 1 of `update_worktree` calls `ensure_bare_clone_github` which fetches origin). 

However, the current order is: fetch (699) → rewrite_authors (703) → update_worktree (711). The rewrite needs `origin/{dest_branch}` to be current. We need to ensure origin is fetched before the rewrite.

**Recommended approach:** Reorder the operations:
1. Call `zbobr.update_worktree(&identity)` first — this fetches origin, syncs branches, and ensures worktree is up-to-date
2. Then call `rewrite_authors_on_worktree` — uses the now-current `origin/{dest_branch}`
3. Then push the result — either via another `update_worktree` call or by adding a dedicated push method

Since `update_worktree` also pushes at the end (Phase 10), calling it before rewrite would push un-rewritten commits. Instead, the simplest correct approach:

**Just delete line 699.** The `update_worktree` at line 711 will:
1. Fetch origin (via `ensure_bare_clone_github`)
2. Push the rewritten commits

The `rewrite_authors_on_worktree` at line 703 uses `origin/{dest_branch}` as the range boundary. The refs in the worktree's bare repo may already be current from a previous `update_worktree` call during the task lifecycle. If they're stale, the rewrite range may be slightly wider than necessary, but it won't produce incorrect results — `filter-branch` rewrites are idempotent for commits that already have the correct author.

So simply removing line 699 is safe and correct.

```diff
-    git(&repo_dir, &["fetch", "origin", dest_branch]).await?;
-
     if !dry_run {
```

Also check if removing this makes the `git` import unused in commands.rs — if `git` is still used elsewhere in the file, keep the import; otherwise update it.
