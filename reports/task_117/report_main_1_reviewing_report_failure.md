## Review: task-117 (do not pass GitHub token in URL)

### Overall assessment
**❌ Does not meet the task requirement yet.** The change removed the token from explicit CLI URL arguments, but the token is still embedded into a URL string via git’s `url.<...>.insteadOf` rewrite (now passed through `GIT_CONFIG_*` environment variables). That means the runtime URL that git uses still contains the token and can still show up in git stderr/log output.

### Must-fix findings
1) **Token still appears in a URL string (core requirement violation)**
- File: `zbobr-repo-backend-github/src/github.rs`
- Code: `token_auth_env()` builds
  - `GIT_CONFIG_KEY_0 = url.https://x-access-token:{token}@github.com/.insteadOf`
- Even though it’s delivered via env, the token is still part of a URL.
- Practical leak path: if a git operation fails, git often prints the full remote URL to stderr (e.g. “fatal: repository 'https://x-access-token:…@github.com/…' not found”), and `git_env()` currently inherits stderr by default.

**Suggested fix:** switch away from URL-rewrite auth to a header/credential-based mechanism that keeps the token out of URLs entirely, e.g. configure `http.https://github.com/.extraheader` (or `http.extraheader`) via `GIT_CONFIG_COUNT/KEY/VALUE` and set an `Authorization: basic <base64(x-access-token:TOKEN)>` header (or another git-supported header scheme for GitHub). This keeps the transport URL clean.

2) **Behavioral change in `overwrite_author`: removed fetch instead of making it auth-safe**
- File: `zbobr/src/commands.rs`
- Change: deleted `git(&repo_dir, &["fetch", "origin", dest_branch]).await?;`
- Risk: `dest_branch` may be stale locally; dry-run output and rewrite base selection can be incorrect.

**Suggested fix:** replace the fetch with an auth-safe alternative (likely by routing through the same backend auth mechanism used elsewhere, or by using the new env-based git helper with appropriate token env for that repository).

### Should-fix / robustness findings
3) **`cleanup_legacy_token_config` silently ignores failures**
- File: `zbobr-repo-backend-github/src/github.rs`
- It drops errors from `git ... --unset`.
- Consider returning `Result<()>` and logging failures, or using `--unset-all` for safety.

4) **Repeated string literals for env keys**
- Strings like `GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_0`, `GIT_CONFIG_VALUE_0` are hardcoded.
- Project rule says avoid repeated string literals; prefer `const` definitions.

### Analog/pattern consistency
No strong existing analog in-repo for git HTTPS auth via env-config was found. The implementation introduces a new approach; however, it currently contradicts the stated goal because it still relies on token-bearing URLs (just moved into env-config).

### Summary
- ✅ Good direction: new `git_env` / `git_check_env` helpers enable env-only auth configuration.
- ❌ Not acceptable yet: token still ends up inside a URL via `insteadOf` rewrite, and git stderr can still leak it.
- ⚠️ `overwrite_author` fetch removal should be replaced with an auth-correct fetch.
