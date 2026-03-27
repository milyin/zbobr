## Review summary
I reviewed all changes on `zbobr_fix-117-pass-token-via-env` vs `origin/main` (3-dot diff). Overall direction is correct: git network operations now use env-injected `http.extraheader` via `GIT_CONFIG_COUNT/KEY/VALUE`, and token-in-URL usage in code appears removed.

However, there is a critical remaining leakage vector that violates the task goal.

---

## Critical issue (must fix)
### 1) Token can still be written to logs
**File:** `zbobr-repo-backend-github/src/github.rs` (cleanup_legacy_token_config)

`cleanup_legacy_token_config()` enumerates legacy `url.*.insteadOf` keys (which historically embed the token in the *key name*) and tries to unset them. On failure it logs:

```rust
tracing::warn!("Failed to remove legacy token config key '{}' in {}: {}", key, ...)
```

If `key` is of the form `url.https://x-access-token:<TOKEN>@github.com/.insteadOf`, the token is emitted into logs — exactly the leak this task is meant to prevent.

**Recommendation:** Never log the raw key. Either:
- log a redacted placeholder (preferred), e.g. `"Failed to remove legacy token config key (redacted)"`, or
- sanitize the key (strip anything between `x-access-token:` and `@github.com`).

Also consider best-effort removal without printing token-bearing data in any error path.

---

## Important improvements (should fix)
### 2) Existing on-disk remotes may still contain token-in-URL
The new code clones with a clean URL and uses headers for auth, which is good. But existing bare repos created before the earlier “normalize origin URL” step could still have `remote.*.url` containing `https://x-access-token:...@github.com/...`.

**Recommendation:** In `ensure_bare_clone_github()`, unconditionally set `remote.origin.url` to the clean URL (and similarly for `fork` if needed) or detect-and-scrub token-bearing URLs. This avoids future leakage when git prints remote URLs in errors.

---

## Consistency / standards notes (nice to have)
### 3) Workspace dependency consistency
`zbobr-repo-backend-github/Cargo.toml` adds `base64 = "0.22"` as a direct dependency. The repo uses `[workspace.dependencies]` for most shared crates.

**Recommendation:** Add `base64` to `Cargo.toml` workspace deps and use `{ workspace = true }` to keep versions centralized.

---

## Analog / pattern consistency
- The `zbobr-utility` additions (`git_env`, `git_check_env`) follow existing patterns (thin wrappers around `tokio::process::Command`) and are used consistently for networked git operations.
- The `http.extraheader` approach is a good fit for the goal (no token in URL/args/on-disk config).

---

## Verdict
❌ **Request changes** due to the logging leak in `cleanup_legacy_token_config()`.

Once that is fixed (and ideally remote URL scrubbing is added), the solution should meet the task requirements.