## File: `zbobr-repo-backend-github/src/github.rs`

### Update `ensure_bare_clone_github` (lines 287-338)

**Current flow:**
1. Clone with token-embedded URL (`https://x-access-token:{token}@github.com/...`)
2. Normalize origin URL to remove token
3. Call `configure_token_auth` to write `insteadOf` to git config
4. Fetch origin (using persistent config for auth)

**New flow:**
1. Build auth env vars via `self.token_auth_env()`
2. Clone with **clean** URL + env vars for auth
3. No URL normalization needed (URL is already clean)
4. Clean up legacy `insteadOf` entries from existing bare repos
5. Fetch origin with env vars

```rust
async fn ensure_bare_clone_github(&self, repo: &GitHubRepo) -> anyhow::Result<PathBuf> {
    let bare_dir = self.backend_config.repos_dir.join(format!("{}.git", repo.bare_dir_name()));
    fs::create_dir_all(&self.backend_config.repos_dir).await?;

    let owned_env = self.token_auth_env();
    let env: Vec<(&str, &str)> = owned_env.iter().map(|(k, v)| (*k, v.as_str())).collect();

    if !bare_dir.exists() {
        let clone_url = format!("https://github.com/{}.git", repo.full_name);
        let bare_name = format!("{}.git", repo.bare_dir_name());
        tracing::info!("Creating bare clone of {} at {}", repo.full_name, bare_dir.display());
        git_env(
            &self.backend_config.repos_dir,
            &["clone", "--bare", &clone_url, &bare_name],
            &env,
        ).await?;
    }

    // Remove legacy token-in-config entries from existing repos
    self.cleanup_legacy_token_config(&bare_dir).await;

    // Configure fetch refspec so worktrees get proper origin/* refs
    git(&bare_dir, &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"]).await?;

    tracing::info!("Fetching origin in {}", bare_dir.display());
    git_env(&bare_dir, &["fetch", "origin"], &env).await?;

    Ok(bare_dir)
}
```

**Key changes:**
- Line 297-301: Replace `format!("https://x-access-token:{token}@github.com/...")` with `format!("https://github.com/...")` 
- Line 308-311: Change `git(...)` to `git_env(..., &env)`
- Line 314-316: Remove the post-clone URL normalization (no longer needed)
- Line 321: Replace `self.configure_token_auth(&bare_dir).await?` with `self.cleanup_legacy_token_config(&bare_dir).await`
- Line 335: Change `git(...)` to `git_env(..., &env)`
