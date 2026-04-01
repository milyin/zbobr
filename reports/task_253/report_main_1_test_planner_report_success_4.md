# Test Plan — Post-fix 578eaae (repo_short_name normalization + branch validation)

## Analysis

The latest working stage (ctx_rec_39) fixed 3 review issues:
1. **GitHub repo_short_name URL normalization** — strip `.git` suffix, trailing slashes
2. **Branch validation** — empty-branch check in both FS and GitHub backend `validate()`
3. **Doc updates** — removed fork_owner, preparator references

## Coverage assessment

| Change | Tested? | Notes |
|--------|---------|-------|
| repo_short_name normalization (FS) | ✅ Yes | 6 tests in `zbobr-repo-backend-fs/src/config.rs` |
| repo_short_name normalization (GitHub) | ✅ Yes | 6 tests in `zbobr-repo-backend-github/src/config.rs` |
| Branch validation (FS) | ❌ No | New `validate()` check, untested |
| Branch validation (GitHub) | ❌ No | New `validate()` check, untested |
| Doc updates | N/A | No tests needed |

## Plan

**1 checklist item**: Add unit tests for `validate()` on both backend configs — 5 tests total covering empty-repository and empty-branch error cases, plus one happy-path test for FS (GitHub happy path requires token setup, skipped).

All 46 existing tests pass (1 pre-existing unrelated failure in `compact_comment_uses_first_line_only` remains).
