# Test: config validate() — branch and repository empty checks

## What to test

The `validate()` method on both `ZbobrRepoBackendFsConfig` and `ZbobrRepoBackendGithubConfig` now checks that `branch` is non-empty (added in 578eaae). These checks are untested.

## Tests to add

### FS backend (`zbobr-repo-backend-fs/src/config.rs`, existing `tests` module)

1. `validate_ok_with_defaults_and_repo` — set repository to a non-empty value, call `validate()`, assert Ok
2. `validate_fails_empty_repository` — default config (empty repository), call `validate()`, assert error contains "repository not set"
3. `validate_fails_empty_branch` — set repository but set branch to `""`, call `validate()`, assert error contains "branch not set"

### GitHub backend (`zbobr-repo-backend-github/src/config.rs`, existing `tests` module)

1. `validate_fails_empty_repository` — default config (empty repository), call `validate()`, assert error contains "repository not set"
2. `validate_fails_empty_branch` — set repository but set branch to `""`, call `validate()`, assert error contains "branch not set"

Note: GitHub's `validate()` takes `&mut self` and proceeds to resolve `github_token` after the branch check, so we can only test the early-bail cases (empty repo, empty branch) without needing a valid token. A full `validate_ok` test would require a resolvable token, so skip that.

## Location

Add tests to the existing `#[cfg(test)] mod tests` blocks in each config module.
