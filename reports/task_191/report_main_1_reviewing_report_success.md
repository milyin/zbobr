# Review Report: Add Issue Link to PR

## Summary
The changes successfully implement the requirement to link the created PR to the corresponding issue in the task backend.

## Verification Checklist

### 1. Implementation of `TaskBackend::task_repo_name`
- **Correctness**: Added `task_repo_name()` to the `TaskBackend` trait with a default `None` implementation. This is non-breaking for other backends.
- **GitHub Backend**: Correctly implemented in `zbobr-task-backend-github/src/github.rs` to return the configured `github_repo` ("owner/repo").
- **Consistency**: Checked that `github_repo` is validated to be in "owner/repo" format.

### 2. Update to `WorktreeBackend::ensure_pr_url`
- **Signature Change**: The method signature now accepts `body: Option<&str>`. This breaking change was correctly propagated to all implementations:
    - `zbobr-repo-backend-github`
    - `zbobr-repo-backend-fs`
    - `zbobr-dispatcher` (mocks)
- **Logic**:
    - In `zbobr-repo-backend-github`, the body is used when creating the PR (`body.unwrap_or("")`).
    - **Note**: The body is only set when *creating* a new PR. Existing PRs are not updated. This satisfies the requirement "the pr created should contain link...".

### 3. Integration in `zbobr-dispatcher`
- **Logic**: The dispatcher correctly retrieves the repo name and constructs the body string: `Resolves https://github.com/{owner}/{repo}/issues/{task_id}`.
- **Robustness**: Uses `Option` chaining (`map`, `as_deref`) to safely handle cases where the task backend doesn't support repo names (e.g., FS backend), resulting in `None` passed to `ensure_pr_url`, which matches previous behavior.
- **URL Validity**: Confirmed that `task_id` in `TaskBackendGithub` corresponds to the issue number, making the URL valid.

### 4. Code Quality
- **Style**: Code follows Rust idioms and project style.
- **Safety**: No unsafe code introduced. Usage of `unwrap_or` is appropriate.

## Conclusion
The implementation is complete, correct, and robust. It effectively links PRs to issues using standard GitHub "Resolves" syntax without breaking existing functionality.
