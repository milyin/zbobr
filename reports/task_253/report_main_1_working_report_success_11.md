# Fix summary for review ctx_rec_85

## Issue 1: GitHub repo normalization too permissive

**Problem**: `parse_github_repo()` in `zbobr-repo-backend-github/src/github.rs` accepted HTTPS URLs from any hostname (e.g., `https://gitlab.com/owner/repo` would pass because it has exactly 5 path components). Also, the plain `owner/repo` format didn't validate non-empty parts (e.g., `/repo` → `["", "repo"]` → len==2 → passed).

**Fix**:
- Added `parts[2] != "github.com"` check to HTTPS URL validation
- Added `parts[0].is_empty() || parts[1].is_empty()` check to final `owner/repo` format validation
- Added 2 new tests: `parse_rejects_non_github_https_url` and `parse_rejects_plain_format_with_empty_parts`

## Issue 2: README CLI flag references incorrect

**Problem**: README referenced `--repo-github-repository` but the actual CLI flag (verified via `zbobr --help`) is `--repo-repository`. Two occurrences in README.md. Also found stale "reference a target repo" wording in the workflow description.

**Fix**:
- `--repo-github-repository` → `--repo-repository` (2 occurrences)
- Removed "and reference a target repo" from step 1 of How It Works workflow

## Test results
- 47 tests pass across all packages
- 1 pre-existing unrelated failure: `compact_comment_uses_first_line_only` in zbobr-api

## Commit
`0bf8441 fix(#253): stricter parse_github_repo validation and fix README CLI flag refs`
