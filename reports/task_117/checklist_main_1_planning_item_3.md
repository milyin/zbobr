## File: `zbobr-repo-backend-github/src/github.rs`

### 1. Update `ensure_fork_remote` (lines 342-361)

Line 358: Change `git(bare_dir, &["fetch", "fork"])` to use `git_env` with auth env.

This method needs the auth env vars. Since it's a `&self` method, compute them inside:
```rust
let owned_env = self.token_auth_env();
let env: Vec<(&str, &str)> = owned_env.iter().map(|(k, v)| (*k, v.as_str())).collect();
// ...
git_env(bare_dir, &["fetch", "fork"], &env).await?;
```

### 2. Update `sync_fork_base_with_upstream` (line 545)

Change `git(bare_dir, &["fetch", "fork"])` to `git_env` with auth env. This is also a `&self` method, same pattern.

### 3. Update `fetch_remote_work_branch` (static method, line 596-612)

This is currently a static method: `async fn fetch_remote_work_branch(bare_dir, push_remote, work_branch)`.

Add an `envs: &[(&str, &str)]` parameter:
```rust
async fn fetch_remote_work_branch(
    bare_dir: &Path,
    push_remote: &str,
    work_branch: &str,
    envs: &[(&str, &str)],
) -> anyhow::Result<bool> {
    let refspec = format!("refs/heads/{work_branch}:refs/remotes/{push_remote}/{work_branch}");
    let ok = git_check_env(bare_dir, &["fetch", push_remote, &refspec], envs).await?;
    // ... rest unchanged
}
```

### 4. Update `push_worktree_to_remote` (static method, lines 670-681)

Add an `envs: &[(&str, &str)]` parameter:
```rust
async fn push_worktree_to_remote(
    worktree_path: &Path,
    push_remote: &str,
    work_branch: &str,
    envs: &[(&str, &str)],
) -> anyhow::Result<()> {
    tracing::info!("Pushing {work_branch} to {push_remote} (no force)");
    git_env(
        worktree_path,
        &["push", push_remote, &format!("HEAD:{work_branch}")],
        envs,
    ).await
}
```

### 5. Update `update_worktree` (lines 760-903)

Compute auth env once at the top of the method, then pass to all remote-contacting calls:

```rust
let owned_env = self.token_auth_env();
let env: Vec<(&str, &str)> = owned_env.iter().map(|(k, v)| (*k, v.as_str())).collect();
```

Update the call sites:
- Line 804: `Self::fetch_remote_work_branch(&bare_dir, &push_remote, work_branch, &env).await?;`
- Line 840: `Self::push_worktree_to_remote(workspace_path, &push_remote, work_branch, &env).await?;`
- Line 899: `Self::push_worktree_to_remote(workspace_path, &push_remote, work_branch, &env).await?;`

Note: `ensure_bare_clone_github` (line 780) and `ensure_fork_remote` (line 790) already handle their own env vars internally.
