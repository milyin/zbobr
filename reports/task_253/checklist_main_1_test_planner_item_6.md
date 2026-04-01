# Add test for `from_config()` repository normalization

**File:** `zbobr-repo-backend-github/src/github.rs`
**Function:** `ZbobrRepoBackendGithub::from_config()` (line ~150)

The latest fix (commit 02e3dec) added normalization of the `repository` field inside `from_config()` so that downstream API calls work regardless of input format. This behavior should have at least one integration-level unit test.

## Tests to add

1. **`from_config_normalizes_https_url`** — Create a `ZbobrRepoBackendGithubConfig` with `repository: "https://github.com/owner/repo.git"`, call `from_config()`, verify that `backend_config.repository` is `"owner/repo"` after construction. Access the normalized value via the struct's `backend_config` field.
2. **`from_config_normalizes_ssh_url`** — Same with `"git@github.com:owner/repo.git"` → `"owner/repo"`.

Note: `from_config()` requires a valid (non-empty) token to build octocrab, so supply a dummy token value. These tests don't make network calls.