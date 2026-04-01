# Fix: Malformed GitHub URL Rejection and README Doc Corrections

## Changes

### 1. `zbobr-repo-backend-github/src/github.rs` — URL validation

Added `is_valid_github_name()` helper that checks owner/repo parts only contain `[a-zA-Z0-9_\-\.]` characters. Applied in two places:

- **HTTPS URL path**: now checks `parts[0] == "https:"` (rejects `http://`) and validates owner/repo parts with `is_valid_github_name`
- **Final validation**: replaced empty-check with `is_valid_github_name` to catch `?`, `#`, and other invalid chars from query strings/fragments

Previously accepted (now rejected):
- `https://github.com/owner/repo?tab=readme-ov-file` → was yielding "owner/repo?tab=readme-ov-file"
- `https://github.com/owner/repo#readme` → was yielding "owner/repo#readme"
- `http://github.com/owner/repo` → non-HTTPS URL

Added 3 new tests: `parse_rejects_url_with_query_string`, `parse_rejects_url_with_fragment`, `parse_rejects_http_url`.

Total GitHub backend tests: 31 (all passing).

### 2. `README.md` — doc corrections

- **Owner Token section**: split into "Repo Token" (manages branches/PRs on code repo via `[repo]` token) and "Task Token" (manages issues/milestones/labels via `[tasks]` token). The old "Owner Token" incorrectly said the `[repo]` token was used for "managing issues, labels, milestones".

- **Stage-specific settings note**: corrected from "placed in nested tables under `[dispatcher]`" to "defined in `[workflow.pipelines.*.stages.*]` tables" which is accurate for the current workflow config design.

## Test results

`cargo test --workspace`: 46 passed, 1 failed (pre-existing `compact_comment_uses_first_line_only` unrelated to this task).