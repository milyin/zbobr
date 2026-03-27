## Scope / diff overview
Compared `origin/main...HEAD`. Changes are focused on removing token-in-URL auth and routing git auth via env:
- Added `git_env` / `git_check_env` helpers (utility).
- GitHub repo backend now builds `GIT_CONFIG_COUNT/KEY/VALUE` env to set `http.https://github.com/.extraheader` for Authorization header.
- Removed URL rewrite (`insteadOf`) auth configuration; added best-effort cleanup of legacy token-bearing `insteadOf` entries.
- Added `fetch_refs` to `WorktreeBackend` and wired through dispatcher + fs/github backends; used by `overwrite_author` to avoid dry-run side effects.
- Centralized `base64` dependency at workspace level.

## Task requirement: “do not pass GitHub token in URL”
✅ Met.
- `ensure_bare_clone_github` now clones from `https://github.com/{full}.git` (no credentials in URL).
- Networked git ops (clone/fetch/push) use `git_env(...)` with a git-config-via-env approach; token is only present in env (`GIT_CONFIG_VALUE_0`), not in URLs or CLI args.

## Analog / pattern consistency
- The previous analog in this module was `configure_token_auth()` using git config URL rewrite. New code keeps the same “configure auth at the git layer” pattern but switches to the safer git-supported mechanism (`GIT_CONFIG_*` + `http.*.extraheader`).
- Use of shared helpers in `zbobr-utility` is consistent with existing patterns (centralized git helpers).

## Token leakage audit
✅ Good improvements:
- No remaining `x-access-token:` URL embedding.
- `cleanup_legacy_token_config()` explicitly avoids calling the `git()` helper for `git config --unset <key>` because that helper would include args in the error context; it also suppresses stderr/stdout and redacts potential credentials in the logged key.
- `git_env` error messages do not include env values.

No new obvious logging paths that would print the token.

## Correctness / robustness notes
- Auth header is set via `Authorization: basic {base64("x-access-token:TOKEN")}`. This is generally correct for GitHub; however, consider using the canonical casing `Authorization: Basic ...` (scheme casing is usually case-insensitive, but canonical improves readability and avoids any edge-case strictness).
- Config key uses `.../.extraheader` (lowercase). Git config keys are case-insensitive, but canonical `extraHeader` could be preferable.
- `token_auth_env()` repeats the env-vector conversion (`owned_env` -> `Vec<(&str,&str)>`) at several call sites; a small helper could reduce duplication (non-blocking).

### fetch_refs / overwrite_author
- `overwrite_author` now calls `zbobr.fetch_refs(&identity)` instead of running `git fetch` directly in the repo dir. This is good for keeping auth consistent and avoiding `update_worktree()` side effects in dry-run.
- Minor potential gap: GitHub `fetch_refs` currently only runs `git fetch origin` in the bare clone; unlike `update_worktree`, it does not sync `refs/heads/{base_branch}` to the freshly fetched remote-tracking ref. If the intent is to make the `{dest_branch}..HEAD` range track the latest base branch, you may want to also update the local base ref (still fetch-only, no merges/pushes/PRs).

## Extraneous changes
No unrelated functional changes observed beyond what’s needed to support env-based auth and side-effect-free fetch.

## Overall assessment
✅ Implementation addresses the security requirement and follows existing repo patterns. Only minor nits/suggestions around canonical header/config casing and an optional robustness improvement for `fetch_refs` base-ref syncing.