# Test Plan — Commit 81fb5d0 (stricter parse_github_repo validation)

## Summary
The latest commit tightened `parse_github_repo()` to reject URLs with extra path segments. The HTTPS rejection path is already tested (`parse_rejects_url_with_extra_path`), but the **SSH rejection path** — which validates that the part after `:` is exactly `owner/repo` — has no test coverage.

## Checklist
1. **ctx_rec_82** — Add `parse_rejects_ssh_url_with_extra_path` test (3 assertions): extra path segments, empty owner, empty repo

## Existing coverage (no changes needed)
- `sanitize_branch_postfix` (11 tests) ✅
- `repo_short_name` FS + GitHub (12 tests) ✅
- `TaskIdentity` (2 tests) ✅
- Preparator removal (2 tests) ✅
- Config validation FS + GitHub (8 tests) ✅
- `parse_github_repo` valid cases (8 tests) ✅
- `parse_github_repo` HTTPS rejection (1 test) ✅
- `from_config` normalization (2 tests) ✅

## Pre-existing unrelated failure
`compact_comment_uses_first_line_only` — not related to this branch.

## All 46 tests pass (1 pre-existing failure unrelated).
