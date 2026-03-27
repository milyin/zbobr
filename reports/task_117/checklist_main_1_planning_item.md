## File: `zbobr-utility/src/lib.rs`

Add three new functions that accept environment variables alongside the existing `git()`, `git_output()`, `git_check()`:

```rust
pub async fn git_env(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> Result<()> {
    let status = tokio::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .envs(envs.iter().copied())
        .status()
        .await
        .with_context(|| format!("Failed to spawn: git {}", args.join(" ")))?;
    if !status.success() {
        anyhow::bail!("git {} failed in {}", args.join(" "), dir.display());
    }
    Ok(())
}

pub async fn git_check_env(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> Result<bool> {
    let status = tokio::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .envs(envs.iter().copied())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .with_context(|| format!("Failed to spawn: git {}", args.join(" ")))?;
    Ok(status.success())
}
```

`git_output_env` is not needed since no remote-contacting call uses `git_output`.

**Optional refactor** to reduce duplication: make the existing `git()`, `git_output()`, `git_check()` delegate to the `_env` variants with empty envs:
```rust
pub async fn git(dir: &Path, args: &[&str]) -> Result<()> {
    git_env(dir, args, &[]).await
}
pub async fn git_check(dir: &Path, args: &[&str]) -> Result<bool> {
    git_check_env(dir, args, &[]).await
}
```
Note: `git_output` does NOT need an `_env` variant so it stays as-is.
