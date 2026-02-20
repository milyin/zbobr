use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

pub mod macros;

/// Resolve a relative path against a base directory; absolute paths are returned as-is.
pub fn resolve_path(path: PathBuf, base: &Path) -> PathBuf {
    if path.is_relative() {
        base.join(path)
    } else {
        path
    }
}

// Replace characters that are unsafe or invalid in filenames with '_'.
// Allows ASCII alphanumerics, '-', '_', and '.'.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Create a placeholder file in a branch to ensure it has at least one commit.
///
/// Behavior mirrors the old implementation in `zbobr-dispatcher`:
/// - Creates `.zbobr/{sanitized_branch}` file
/// - Stages it with `git add` and commits it with `git commit`
///
/// Errors include helpful context for diagnostics.
pub async fn create_placeholder_commit(work_dir: &Path, branch_name: &str) -> Result<()> {
    let zbobr_dir = work_dir.join(".zbobr");
    let sanitized_branch = sanitize_filename(branch_name);
    let placeholder_path = zbobr_dir.join(&sanitized_branch);

    // Create .zbobr directory
    tokio::fs::create_dir_all(&zbobr_dir)
        .await
        .map_err(|e| anyhow!("Failed to create .zbobr directory: {}", e))?;

    // Create placeholder file with extended diagnostics on failure
    match tokio::fs::File::create(&placeholder_path).await {
        Ok(_) => {}
        Err(e) => {
            let kind = e.kind();
            let raw = e.raw_os_error();

            let zbobr_exists = tokio::fs::metadata(&zbobr_dir).await.is_ok();
            let work_dir_meta = tokio::fs::metadata(work_dir).await;
            let work_dir_readonly = work_dir_meta
                .as_ref()
                .map(|m| m.permissions().readonly())
                .unwrap_or(false);

            anyhow::bail!(
                "Failed to create placeholder file: {} — attempted path: {} — work_dir: {} — .zbobr exists: {} — work_dir_readonly: {} — kind: {:?} — raw_os_error: {:?}",
                e,
                placeholder_path.display(),
                work_dir.display(),
                zbobr_exists,
                work_dir_readonly,
                kind,
                raw
            );
        }
    }

    // Stage the file
    let add_status = tokio::process::Command::new("git")
        .args(["add", &format!(".zbobr/{}", sanitized_branch)])
        .current_dir(work_dir)
        .status()
        .await
        .map_err(|e| anyhow!("Failed to run git add: {}", e))?;

    if !add_status.success() {
        anyhow::bail!("git add for placeholder failed (exit != 0)");
    }

    // Commit the file
    let commit_msg = format!("chore: add branch placeholder {}", branch_name);
    let commit_status = tokio::process::Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(work_dir)
        .status()
        .await
        .map_err(|e| anyhow!("Failed to run git commit: {}", e))?;

    if !commit_status.success() {
        anyhow::bail!("git commit for placeholder failed (exit != 0)");
    }

    Ok(())
}
