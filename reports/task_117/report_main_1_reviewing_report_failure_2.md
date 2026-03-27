## Summary
Goal: stop embedding GitHub token in URLs; pass via env. Most changes do this correctly (http.extraheader via `GIT_CONFIG_*` env), but two issues remain.

## Findings (must fix)

### 1) Token can still leak via `cleanup_legacy_token_config()` error logging
File: `zbobr-repo-backend-github/src/github.rs`

You redact the config key (`redacted_key`), but the logged error `e` is produced by `zbobr_utility::git_env()` / `git()` which bails with:
- `anyhow::bail!("git {} failed in {}", args.join(" "), dir.display());`

When `args` contains `--unset <key>`, `<key>` may include the legacy token (`url.https://x-access-token:<TOKEN>@github.com/.insteadOf`). If `git config --unset` fails (lock, permissions, etc.), `e` will include the full args and therefore the token. This reintroduces the exact leakage risk the task aims to remove.

**Recommended fix:** In `cleanup_legacy_token_config()`, do not call the generic `git()` helper for unsetting legacy keys. Instead, run `tokio::process::Command` directly and on failure log only sanitized key + exit status (and suppress stderr). Alternatively, avoid logging `e` entirely, or sanitize `e` before logging.

### 2) `overwrite_author()` dry-run behavior is no longer read-only
File: `zbobr/src/commands.rs`

`git fetch origin <dest_branch>` was replaced with:
```rs
zbobr.update_worktree(&identity).await?;
```
But `update_worktree()` in the GitHub backend performs a full merge-based sync and includes pushes, placeholder commit creation, and PR ensuring logic. This can:
- modify the worktree (stash/merge),
- create commits/branches/PRs,
- push to remotes,
including when `dry_run == true`.

That’s a functional regression and surprising for a CLI command that previously only fetched refs.

**Recommended fix:** restore a narrow “auth-safe fetch” for `dest_branch` without merges/push/PR side effects. Ideally reuse the same env-based auth mechanism (extraheader via `GIT_CONFIG_*`) but keep the operation limited to fetch.

## Additional notes (nice-to-have)
- Consider using `const` for repeated env keys (`GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_0`, etc.) to reduce divergence risk.
- `token_auth_env()` duplicates env Vec construction in multiple call sites; could be a helper returning `Vec<(&str,&str)>` scoped to a call.

## Analog / consistency
Overall style matches existing async git helpers (`zbobr_utility`), but the two issues above diverge from expected safety guarantees (no secret logging, dry-run shouldn’t have side effects).

## Verdict
Fail until (1) error logging is fully secret-safe, and (2) overwrite_author dry-run is side-effect free / fetch-only.