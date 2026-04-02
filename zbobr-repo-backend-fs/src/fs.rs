use std::path::Path;

use anyhow::Context;
use async_trait::async_trait;
use tokio::fs;
use zbobr_api::{backend::WorktreeBackend, task::TaskIdentity};
use zbobr_utility::{git, git_check, git_output};

use crate::config::ZbobrRepoBackendFsConfig;

/// Filesystem-based repo backend using bare clones and git worktrees.
///
/// - Bare clones are stored at `repos_dir/repo_name.git`
/// - Worktrees are created via `git worktree add` pointing to the bare clone
/// - Multiple tasks can share the same bare clone
#[derive(Clone)]
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

    /// Ensure bare clone exists at `bare_dir`, fetching latest refs from origin.
    /// Configures fetch refspec so `refs/remotes/origin/*` are available in worktrees.
    async fn ensure_bare_clone(&self, bare_dir: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(&self.config.repos_dir).await?;

        if !bare_dir.exists() {
            tracing::info!("Creating bare clone at {}", bare_dir.display());
            git(
                &self.config.repos_dir,
                &[
                    "clone",
                    "--bare",
                    &self.config.repository,
                    bare_dir.to_str().unwrap(),
                ],
            )
            .await?;
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

        zbobr_utility::cleanup_worktree_for_branch(bare_dir, work_branch, workspace_path).await?;

        let ws = workspace_path.to_str().unwrap();
        let local_work_branch_exists = git_check(
            bare_dir,
            &["rev-parse", &format!("{}^{{commit}}", work_branch)],
        )
        .await?;
        if local_work_branch_exists {
            git(bare_dir, &["worktree", "add", ws, work_branch]).await?;
        } else {
            let remote_work_branch = format!("origin/{work_branch}");
            let remote_base_branch = format!("origin/{base_branch}");
            let start_point = if git_check(
                bare_dir,
                &["rev-parse", &format!("{}^{{commit}}", remote_work_branch)],
            )
            .await?
            {
                remote_work_branch.as_str()
            } else {
                remote_base_branch.as_str()
            };
            git(
                bare_dir,
                &["worktree", "add", "-b", work_branch, ws, start_point],
            )
            .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl WorktreeBackend for ZbobrRepoBackendFs {
    fn repository(&self) -> &str {
        &self.config.repository
    }

    fn branch(&self) -> &str {
        &self.config.branch
    }

    fn repo_name(&self) -> &str {
        self.config.repo_short_name()
    }

    async fn update_worktree(
        &self,
        identity: &TaskIdentity,
        workspace_path: &Path,
        _git_user_name: &str,
        _git_user_email: &str,
    ) -> anyhow::Result<bool> {
        let base_branch = &self.config.branch;
        let work_branch = &identity.work_branch;

        if work_branch == base_branch {
            anyhow::bail!(
                "work_branch and base_branch must differ, got '{}'",
                work_branch
            );
        }

        let bare_dir = self
            .config
            .repos_dir
            .join(format!("{}.git", self.config.repo_short_name()));

        self.ensure_bare_clone(&bare_dir).await?;
        self.ensure_worktree(&bare_dir, base_branch, work_branch, workspace_path)
            .await?;

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

    async fn fetch_refs(&self, _identity: &TaskIdentity) -> anyhow::Result<()> {
        let bare_dir = self
            .config
            .repos_dir
            .join(format!("{}.git", self.config.repo_short_name()));
        self.ensure_bare_clone(&bare_dir).await?;
        zbobr_utility::git(&bare_dir, &["fetch", "origin"]).await?;
        Ok(())
    }

    async fn ensure_pr_url(
        &self,
        identity: &TaskIdentity,
        _body: Option<&str>,
    ) -> anyhow::Result<String> {
        let work_branch = &identity.work_branch;

        let bare_dir = self
            .config
            .repos_dir
            .join(format!("{}.git", self.config.repo_short_name()));

        if !bare_dir.exists() {
            anyhow::bail!("No worktree found for work_branch '{}'", work_branch);
        }

        let output = git_output(&bare_dir, &["worktree", "list", "--porcelain"])
            .await
            .context("Failed to list worktrees")?;

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
            if branch.as_deref() == Some(work_branch)
                && let Some(p) = wt_path
            {
                return Ok(p);
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
            "FilesystemRepoBackend(repository={}, branch={}, repos_dir={})",
            self.config.repository,
            self.config.branch,
            self.config.repos_dir.display()
        )
    }
}
