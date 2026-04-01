# Add unit tests for `parse_github_repo()`

**File:** `zbobr-repo-backend-github/src/github.rs`
**Function:** `parse_github_repo()` (line ~110)

This internal function normalizes various GitHub repository reference formats to `owner/repo`. It was the root cause of multiple review failures (ctx_rec_35, ctx_rec_43) and currently has zero test coverage.

## Tests to add

Add a `#[cfg(test)] mod tests` block (or add to existing) in `github.rs` with the following test cases:

1. **`parse_github_repo_bare_owner_repo`** — `"owner/repo"` → `full_name == "owner/repo"`
2. **`parse_github_repo_https_url`** — `"https://github.com/owner/repo"` → `"owner/repo"`
3. **`parse_github_repo_https_url_with_git_suffix`** — `"https://github.com/owner/repo.git"` → `"owner/repo"`
4. **`parse_github_repo_ssh_url`** — `"git@github.com:owner/repo"` → `"owner/repo"`
5. **`parse_github_repo_ssh_url_with_git_suffix`** — `"git@github.com:owner/repo.git"` → `"owner/repo"`
6. **`parse_github_repo_trailing_slash`** — `"owner/repo/"` → `"owner/repo"`
7. **`parse_github_repo_invalid_single_segment`** — `"just-a-name"` → error
8. **`parse_github_repo_invalid_empty`** — `""` → error
9. **`parse_github_repo_invalid_url_no_path`** — `"https://github.com"` → error (or single segment)

These are pure unit tests with no I/O or async needed.