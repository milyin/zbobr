# Test Plan — Round 6

## Summary

The latest implementation changes (commits 578eaae, 02e3dec) added GitHub repository normalization in two places:
1. `parse_github_repo()` — internal function that converts HTTPS URLs, SSH URLs, and bare `owner/repo` to normalized `owner/repo` format
2. `from_config()` — now calls `parse_github_repo()` to normalize the configured repository

These were the root cause of review failures ctx_rec_35 and ctx_rec_43. The function currently has **zero test coverage**.

## Checklist items

1. **ctx_rec_48: Unit tests for `parse_github_repo()`** — 9 test cases covering bare owner/repo, HTTPS URL, SSH URL, .git suffix stripping, trailing slash, and error cases (single segment, empty, URL without path).

2. **ctx_rec_49: Tests for `from_config()` normalization** — 2 test cases verifying that `from_config()` normalizes HTTPS and SSH URLs to `owner/repo` in the stored config.

## What doesn't need new tests

- `sanitize_branch_postfix()` — already has 11 tests including Unicode truncation fix
- `repo_short_name()` — already has 6+5 tests on FS and GitHub backends
- `validate()` — already has 5+ tests on both backends
- `TaskIdentity` — already has 2 tests
- Preparator removal — already has 1 test
- Test wiring fix (target_repo in init_github_github) — covered by existing integration tests
- Doc/README changes — no tests needed

## Pre-existing issue

1 pre-existing test failure (`compact_comment_uses_first_line_only`) is unrelated to this branch.