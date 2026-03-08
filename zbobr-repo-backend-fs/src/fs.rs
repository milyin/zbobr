use std::path::Path;

use anyhow::Context;
use async_trait::async_trait;
use tokio::fs;
use zbobr_api::backend::WorktreeBackend;
use zbobr_utility::{git, git_check, git_output};

use crate::config::ZbobrRepoBackendFsConfig;

/// Filesystem-based repo backend using bare clones and git worktrees.
///
/// - Bare clones are stored at `repos_dir/repo_name.git`
/// - Worktrees are created via `git worktree add` pointing to the bare clone
/// - Multiple tasks can share the same bare clone
pub struct ZbobrRepoBackendFs {
    config: ZbobrRepoBackendFsConfig,
}

impl ZbobrRepoBackendFs {
    pub fn new(
        toml: Option<crate::config::ZbobrRepoBackendFsToml>,
        args: crate::config::ZbobrRepoBackendFsArgs,
        config_dir: &std::path::Path,
    ) -> anyhow::Result<Self> {
        let config =
            <ZbobrRepoBackendFsConfig as zbobr_api::config::Config>::build(toml, args, config_dir);
        Self::from_config(config)
    }

    pub fn from_config(config: ZbobrRepoBackendFsConfig) -> anyhow::Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Extract a short repo name from a remote path (last path component).
    fn repo_name_from_path(target_repo: &str) -> anyhow::Result<String> {
        let path = Path::new(target_repo);
        let name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            anyhow::anyhow!("Cannot extract repo name from path: {}", target_repo)
        })?;
        Ok(name.to_string())
    }

    /// Ensure bare clone exists at `bare_dir`, fetching latest refs from origin.
    /// Configures fetch refspec so `refs/remotes/origin/*` are available in worktrees.
    async fn ensure_bare_clone(&self, remote_repo: &str, bare_dir: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(&self.config.repos_dir).await?;

        if !bare_dir.exists() {
            tracing::info!("Creating bare clone at {}", bare_dir.display());
            let status = tokio::process::Command::new("git")
                .args([
                    "clone",
                    "--bare",
                    remote_repo,
                    bare_dir.to_str().unwrap(),
                ])
                .status()
                .await?;
            if !status.success() {
                anyhow::bail!("Failed to create bare clone from {}", remote_repo);
            }
            // Configure fetch refspec so worktrees get proper origin/* refs
            git(
                bare_dir,
                &[
                    "config",
                    "remote.origin.fetch",
                    "+refs/heads/*:refs/remotes/origin/*",
                ],
            )
            .await?;
        }

        tracing::info!("Fetching origin in {}", bare_dir.display());
        git(bare_dir, &["fetch", "origin"]).await?;
        Ok(())
    }

    /// Create a worktree at `workspace_path` for `work_branch` from `base_branch`.
    async fn ensure_worktree(
        &self,
        bare_dir: &Path,
        base_branch: &str,
        work_branch: &str,
        workspace_path: &Path,
    ) -> anyhow::Result<()> {
        if workspace_path.exists() {
            tracing::info!("Worktree already exists at {}", workspace_path.display());
            return Ok(());
        }

        let workspace_parent = workspace_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot get parent of workspace_path"))?;
        fs::create_dir_all(workspace_parent).await?;

        tracing::info!(
            "Creating worktree for {} at {}",
            work_branch,
            workspace_path.display()
        );

        let ws = workspace_path.to_str().unwrap();
        if work_branch == base_branch {
            git(bare_dir, &["worktree", "add", ws, base_branch]).await?;
        } else if git_check(bare_dir, &["rev-parse", &format!("{}^{{commit}}", work_branch)])
            .await?
        {
            git(bare_dir, &["worktree", "add", ws, work_branch]).await?;
        } else {
            git(
                bare_dir,
                &["worktree", "add", "-b", work_branch, ws, base_branch],
            )
            .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl WorktreeBackend for ZbobrRepoBackendFs {
    async fn update_worktree(
        &self,
        remote_repo: &str,
        base_branch: &str,
        work_branch: &str,
        workspace_path: &Path,
    ) -> anyhow::Result<bool> {
        let repo_name = Self::repo_name_from_path(remote_repo)?;
        let bare_dir = self.config.repos_dir.join(format!("{}.git", repo_name));

        self.ensure_bare_clone(remote_repo, &bare_dir).await?;
        self.ensure_worktree(&bare_dir, base_branch, work_branch, workspace_path)
            .await?;

        // Check if work_branch includes all commits from base_branch
        let is_uptodate = git_check(
            &bare_dir,
            &[
                "merge-base",
                "--is-ancestor",
                &format!("origin/{}", base_branch),
                work_branch,
            ],
        )
        .await?;

        tracing::info!(
            "Worktree {}: {}",
            work_branch,
            if is_uptodate {
                "up-to-date"
            } else {
                "diverged from base_branch"
            }
        );

        Ok(is_uptodate)
    }

    async fn update_pr(&self, work_branch: &str) -> anyhow::Result<String> {
        if !self.config.repos_dir.exists() {
            anyhow::bail!("No worktree found for work_branch '{}'", work_branch);
        }

        let mut entries = fs::read_dir(&self.config.repos_dir)
            .await
            .context("Failed to read repos_dir")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir()
                || !path
                    .file_name()
                    .map_or(false, |n| n.to_string_lossy().ends_with(".git"))
            {
                continue;
            }

            let output = match git_output(&path, &["worktree", "list", "--porcelain"]).await {
                Ok(o) => o,
                Err(_) => continue,
            };

            // Porcelain format: blocks separated by blank lines, each starting with "worktree <path>"
            // followed by "branch refs/heads/<name>"
            for block in output.split("\n\n") {
                let mut wt_path = None;
                let mut branch = None;
                for line in block.lines() {
                    if let Some(p) = line.strip_prefix("worktree ") {
                        wt_path = Some(p.to_string());
                    }
                    if let Some(b) = line.strip_prefix("branch refs/heads/") {
                        branch = Some(b.to_string());
                    }
                }
                if branch.as_deref() == Some(work_branch) {
                    if let Some(p) = wt_path {
                        return Ok(p);
                    }
                }
            }
        }

        anyhow::bail!("No worktree found for work_branch '{}'", work_branch)
    }

    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        fs::create_dir_all(&self.config.repos_dir)
            .await
            .with_context(|| {
                format!(
                    "Cannot access repos directory '{}'",
                    self.config.repos_dir.display()
                )
            })?;

        let test_path = self.config.repos_dir.join(".test");
        fs::write(&test_path, "test").await.with_context(|| {
            format!(
                "Cannot write to repos directory '{}'",
                self.config.repos_dir.display()
            )
        })?;
        let _ = fs::remove_file(&test_path).await;

        tracing::info!("Filesystem repo backend connectivity validated");
        Ok(())
    }

    fn debug_state(&self) -> String {
        format!(
            "FilesystemRepoBackend(repos_dir: {})",
            self.config.repos_dir.display()
        )
    }
}
