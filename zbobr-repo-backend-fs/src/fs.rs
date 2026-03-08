use std::path::{Path, PathBuf};

use anyhow::Context;
use async_trait::async_trait;
use tokio::fs;
use zbobr_api::backend::WorktreeBackend;

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
}

#[async_trait]
impl WorktreeBackend for ZbobrRepoBackendFs {
    /// Prepare a worktree for the given task.
    ///
    /// 1. Creates or updates a bare clone at `repos_dir/repo_name.git`
    /// 2. Creates a worktree at `workspace_path` via `git worktree add`
    /// 3. Returns Ok(true) if the worktree is up-to-date, Ok(false) if base_branch diverged
    async fn update_worktree(
        &self,
        remote_repo: &str,
        base_branch: &str,
        work_branch: &str,
        workspace_path: &Path,
    ) -> anyhow::Result<bool> {
        let repo_name = Self::repo_name_from_path(remote_repo)?;
        let bare_dir = self.config.repos_dir.join(format!("{}.git", repo_name));

        // Ensure parent directory exists
        fs::create_dir_all(&self.config.repos_dir).await?;

        // Step 1: Create or update the bare clone
        if !bare_dir.exists() {
            tracing::info!("Creating bare clone at {}", bare_dir.display());
            let status = tokio::process::Command::new("git")
                .args(["clone", "--bare", remote_repo, bare_dir.to_str().unwrap()])
                .status()
                .await?;
            if !status.success() {
                anyhow::bail!("Failed to create bare clone from {}", remote_repo);
            }
        } else {
            tracing::info!("Updating bare clone at {}", bare_dir.display());
            let status = tokio::process::Command::new("git")
                .args(["fetch", "origin"])
                .current_dir(&bare_dir)
                .status()
                .await?;
            if !status.success() {
                anyhow::bail!("Failed to fetch in bare clone at {}", bare_dir.display());
            }
        }

        // Step 2: Force-update the base_branch ref from origin
        tracing::info!("Updating {} to point to origin/{}", base_branch, base_branch);
        let update_status = tokio::process::Command::new("git")
            .args([
                "fetch", "origin",
                &format!("+refs/heads/{0}:refs/heads/{0}", base_branch),
            ])
            .current_dir(&bare_dir)
            .status()
            .await?;
        if !update_status.success() {
            anyhow::bail!(
                "Failed to update base_branch '{}' from origin at {}",
                base_branch,
                bare_dir.display()
            );
        }

        // Step 3: Ensure workspace_path parent exists
        let workspace_parent = workspace_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot get parent of workspace_path"))?;
        fs::create_dir_all(workspace_parent).await?;

        // Step 4: Create or reuse the worktree
        if workspace_path.exists() {
            tracing::info!("Worktree already exists at {}", workspace_path.display());
            // Validate the worktree is for the correct branch
        } else {
            tracing::info!(
                "Creating worktree for {} at {}",
                work_branch,
                workspace_path.display()
            );

            // Try to create worktree checking out work_branch directly
            let mut create_cmd = tokio::process::Command::new("git");
            create_cmd.args(["worktree", "add", workspace_path.to_str().unwrap()]);

            if work_branch == base_branch {
                // Same branch: check out base_branch directly
                create_cmd.arg(base_branch);
            } else {
                // Different branch: create work_branch from base_branch if it doesn't exist
                // Try checking out work_branch (if it exists on the remote)
                let check_status = tokio::process::Command::new("git")
                    .args(["rev-parse", &format!("{}^{{commit}}", work_branch)])
                    .current_dir(&bare_dir)
                    .status()
                    .await?;
                if check_status.success() {
                    // work_branch exists, check it out
                    create_cmd.arg(work_branch);
                } else {
                    // work_branch doesn't exist, create from base_branch
                    create_cmd.args(["-b", work_branch, base_branch]);
                }
            }

            let status = create_cmd.status().await?;
            if !status.success() {
                anyhow::bail!(
                    "Failed to create worktree at {}",
                    workspace_path.display()
                );
            }
        }

        // Step 5: Check if work_branch is up-to-date with base_branch
        let is_ancestor_status = tokio::process::Command::new("git")
            .args([
                "-C",
                bare_dir.to_str().unwrap(),
                "merge-base",
                "--is-ancestor",
                base_branch,
                work_branch,
            ])
            .status()
            .await?;

        // merge-base --is-ancestor returns exit code 0 if ancestor, 1 otherwise
        let is_uptodate = is_ancestor_status.code() == Some(0);
        if is_uptodate {
            tracing::info!("Worktree {} is up-to-date with base_branch", work_branch);
        } else {
            tracing::info!("Worktree {} diverged from base_branch", work_branch);
        }

        Ok(is_uptodate)
    }

    /// Return the path/URL of the PR for the given work branch.
    ///
    /// For FS backend: finds the worktree for this branch via `git worktree list`
    /// and returns the path.
    async fn update_pr(&self, work_branch: &str) -> anyhow::Result<String> {
        // Try to find a worktree for this work_branch by searching all existing worktrees
        // We'll look in all bare clones under repos_dir
        let entries = fs::read_dir(&self.config.repos_dir)
            .await
            .context("Failed to read repos_dir")?;

        let mut found_worktree = None;

        let mut dir_entries = vec![];
        let mut entries = entries;
        loop {
            match entries.next_entry().await {
                Ok(Some(entry)) => dir_entries.push(entry),
                Ok(None) => break,
                Err(_) => break,
            }
        }

        for entry in dir_entries {
            let path = entry.path();
            if path.is_dir() && path.file_name().map_or(false, |n| n.to_string_lossy().ends_with(".git")) {
                // List worktrees in this bare clone
                let output = tokio::process::Command::new("git")
                    .args(["worktree", "list", "--porcelain"])
                    .current_dir(&path)
                    .output()
                    .await
                    .ok();

                if let Some(output) = output {
                    let worktree_list = String::from_utf8_lossy(&output.stdout);
                    for line in worktree_list.lines() {
                        // Format: "worktree /path/to/worktree"
                        if let Some(worktree_path) = line.strip_prefix("worktree ") {
                            // Check if this worktree is for our work_branch
                            let branch_output = tokio::process::Command::new("git")
                                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                                .current_dir(worktree_path)
                                .output()
                                .await
                                .ok();

                            if let Some(branch_output) = branch_output {
                                let current_branch = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();
                                if current_branch == work_branch {
                                    found_worktree = Some(worktree_path.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }
                if found_worktree.is_some() {
                    break;
                }
            }
        }

        found_worktree.ok_or_else(|| {
            anyhow::anyhow!("No worktree found for work_branch '{}'", work_branch)
        })
    }

    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        // Check that we can write to repos_dir
        fs::create_dir_all(&self.config.repos_dir)
            .await
            .with_context(|| {
                format!(
                    "Cannot access repos directory '{}'",
                    self.config.repos_dir.display()
                )
            })?;

        // Try to write a test file
        let test_path = self.config.repos_dir.join(".test");
        fs::write(&test_path, "test").await.with_context(|| {
            format!(
                "Cannot write to repos directory '{}'",
                self.config.repos_dir.display()
            )
        })?;

        // Clean up
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
