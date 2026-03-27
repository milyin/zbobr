## File: `zbobr-repo-backend-github/src/github.rs`

### 1. Add `token_auth_env` method (private helper on `ZbobrRepoBackendGithub`)

Returns env vars for git's environment-based config override that performs the same URL rewrite as the old `insteadOf` config, but without persisting the token:

```rust
/// Build environment variables that configure git token auth via
/// `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_*` / `GIT_CONFIG_VALUE_*`.
/// The token never appears in command-line args or on-disk config.
fn token_auth_env(&self) -> [(&str, String); 3] {
    let token = &self.backend_config.github_token;
    [
        ("GIT_CONFIG_COUNT", "1".into()),
        (
            "GIT_CONFIG_KEY_0",
            format!("url.https://x-access-token:{token}@github.com/.insteadOf"),
        ),
        ("GIT_CONFIG_VALUE_0", "https://github.com/".into()),
    ]
}
```

At call sites, convert to `&[(&str, &str)]` for the `_env` helpers:
```rust
let owned_env = self.token_auth_env();
let env: Vec<(&str, &str)> = owned_env.iter().map(|(k, v)| (*k, v.as_str())).collect();
```

### 2. Replace `configure_token_auth` (lines 258-285) with `cleanup_legacy_token_config`

Delete `configure_token_auth` entirely. Add a new method that only removes stale `insteadOf` entries from existing bare repos (so old repos don't keep the token in their on-disk git config):

```rust
/// Remove legacy insteadOf entries that embedded the token in git config.
async fn cleanup_legacy_token_config(&self, bare_dir: &Path) {
    if let Ok(output) = git_output(
        bare_dir,
        &["config", "--get-regexp", r"url\..*github\.com.*\.insteadOf"],
    ).await {
        for line in output.lines() {
            if let Some(key) = line.split_whitespace().next() {
                let _ = git(bare_dir, &["config", "--unset", key]).await;
            }
        }
    }
}
```

### 3. Update import line (line 10)

```rust
use zbobr_utility::{git, git_check, git_check_env, git_env, git_output};
```
