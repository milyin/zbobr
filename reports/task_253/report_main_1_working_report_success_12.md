# Fix: SSH Host Validation in parse_github_repo()

## Issue (ctx_rec_91)
The SSH URL branch in `parse_github_repo()` validated only that the path after `:` was `owner/repo` format, but did not check that the host was `git@github.com`. This allowed non-GitHub remotes like `git@gitlab.com:owner/repo` to be accepted.

## Fix Applied (commit 896e87e)

**File:** `zbobr-repo-backend-github/src/github.rs`

Changed the SSH URL handling to:
1. Split on the first `:` to separate host and path
2. Validate that the host portion is exactly `git@github.com`
3. Reject with a clear error message if it's any other host

**New test added:** `parse_rejects_non_github_ssh_host` — tests that `git@gitlab.com:owner/repo` and `git@bitbucket.org:owner/repo` are both rejected with "Invalid GitHub SSH URL" error.

## Test Results
- 28/28 tests pass in `zbobr-repo-backend-github`
- 46/47 workspace tests pass (1 pre-existing unrelated failure in `compact_comment_uses_first_line_only`)