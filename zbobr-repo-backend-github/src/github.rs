use std::{path::PathBuf, time::Duration};

use anyhow::Context;
use async_trait::async_trait;
use tokio::fs;
use zbobr_api::backend::RepoBackend;
use zbobr_utility::{git, git_check, git_output};

use crate::config::ZbobrRepoBackendGithubConfig;

/// Convert an octocrab error into an anyhow::Error with detailed information.
fn octocrab_to_anyhow(e: octocrab::Error) -> anyhow::Error {
    match e {
        octocrab::Error::GitHub { source, .. } => {
            anyhow::anyhow!(
                "GitHub API error: {} (status: {}) -- details: {:?}",
                source.message,
                source.status_code,
                source
            )
        }
        other => anyhow::anyhow!("GitHub API error: {:?}", other),
    }
}

fn is_transient_octocrab_error(error: &octocrab::Error) -> bool {
    match error {
        octocrab::Error::GitHub { source, .. } => source.status_code.is_server_error(),
        _ => true,
    }
}

/// Retry a GitHub API operation up to 3 times on transient errors.
async fn retry_github<T, F, Fut>(op_name: &str, mut f: F) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, octocrab::Error>>,
{
    let mut attempt = 0u64;
    loop {
        attempt += 1;
        match f().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                if attempt < 3 && is_transient_octocrab_error(&e) {
                    tracing::warn!(
                        "Transient GitHub error during {op_name} (attempt {attempt}/3): {e}"
                    );
                    tokio::time::sleep(Duration::from_millis(250 * attempt)).await;
                    continue;
                }
                return Err(octocrab_to_anyhow(e));
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct RepoResponse {
    full_name: String,
}

#[derive(Debug)]
struct GitHubRepo {
    full_name: String,
}

impl GitHubRepo {
    /// Returns the owner part of the repository (before the '/').
    fn owner(&self) -> &str {
        self.full_name.split('/').next().unwrap_or("")
    }

    /// Returns the name part of the repository (after the '/').
    fn name(&self) -> &str {
        self.full_name.split('/').nth(1).unwrap_or("")
    }
}

fn parse_github_repo(repo_ref: &str) -> anyhow::Result<GitHubRepo> {
    // Standardize: remove trailing .git and /
    let repo_ref = repo_ref.trim_end_matches(".git").trim_end_matches('/');

    let full_name = if repo_ref.contains("://") {
        // extract owner/repo from URL
        let parts: Vec<&str> = repo_ref.split('/').collect();
        if parts.len() < 2 {
            anyhow::bail!("Invalid GitHub URL: {}", repo_ref);
        }
        format!("{}/{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else if repo_ref.contains(':') {
        // git@github.com:owner/repo
        repo_ref.rsplit(':').next().unwrap_or("").to_string()
    } else {
        repo_ref.to_string()
    };

    let parts: Vec<&str> = full_name.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!(
            "Invalid GitHub repository format: {}. Expected 'owner/repo' or a GitHub URL.",
            repo_ref
        );
    }

    Ok(GitHubRepo { full_name })
}

// ============================================================================
// ZbobrRepoBackendGithub
// ============================================================================

pub struct ZbobrRepoBackendGithub {
    backend_config: ZbobrRepoBackendGithubConfig,
    octocrab: octocrab::Octocrab,
    git_user_name: String,
    git_user_email: String,
}

impl ZbobrRepoBackendGithub {
    pub fn new(
        toml: Option<crate::config::ZbobrRepoBackendGithubToml>,
        args: crate::config::ZbobrRepoBackendGithubArgs,
        git_user_name: String,
        git_user_email: String,
    ) -> anyhow::Result<Self> {
        let backend_config = <ZbobrRepoBackendGithubConfig as zbobr_api::config::Config>::build(
            toml,
            args,
            std::path::Path::new("."),
        );
        Self::from_config(backend_config, git_user_name, git_user_email)
    }

    pub fn from_config(
        backend_config: ZbobrRepoBackendGithubConfig,
        git_user_name: String,
        git_user_email: String,
    ) -> anyhow::Result<Self> {
        backend_config.validate()?;
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(backend_config.github_token.clone())
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build octocrab client: {e}"))?;
        Ok(Self {
            backend_config,
            octocrab,
            git_user_name,
            git_user_email,
        })
    }

    async fn ensure_fork(
        &self,
        target_repo: &str,
        destination_branch: &str,
    ) -> anyhow::Result<String> {
        let repo = parse_github_repo(target_repo)?;
        let fork_repo = format!("{}/{}", self.backend_config.fork_owner, repo.name());

        // Check if fork already exists
        let exists = retry_github("check fork exists", || {
            self.octocrab
                .get::<RepoResponse, _, _>(format!("/repos/{fork_repo}"), None::<&()>)
        })
        .await
        .is_ok();

        if !exists {
            let fork_owner = &self.backend_config.fork_owner;
            let endpoint = format!("/repos/{}/forks", repo.full_name);
            let payload = serde_json::json!({ "organization": fork_owner });

            tracing::info!(
                "Creating fork of {target_repo} under organization '{fork_owner}' using endpoint {endpoint}"
            );
            tracing::debug!("Fork creation payload: {payload}");

            retry_github("create fork", || {
                self.octocrab.post(&endpoint, Some(&payload))
            })
            .await
            .map_err(|e| {
                let error_details = format!("{:?}", e);
                tracing::error!(
                    "Failed to create fork: target_repo={}, fork_owner={}, endpoint={}, error={:?}",
                    target_repo,
                    fork_owner,
                    endpoint,
                    e
                );
                anyhow::anyhow!(
                    "Failed to create fork of {target_repo} under '{fork_owner}': \
                         check if fork_owner is an organization you have access to, \
                         and that your GitHub token has 'repo' and 'admin:org_hook' scopes. \
                         Endpoint: {endpoint}. Error: {e}\n\
                         Debug: {error_details}",
                )
            })
            .map(|_: serde_json::Value| ())?;

            // Wait a moment for the fork to be ready
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        } else {
            // Fork exists — sync it with the destination branch on upstream.
            // The destination branch (e.g. main) must exist on upstream; if it
            // somehow doesn't, that is an error we should surface.
            let endpoint = format!("/repos/{}/merge-upstream", fork_repo);
            let body = serde_json::json!({ "branch": destination_branch });

            tracing::info!(
                "Syncing fork {fork_repo} with upstream {}/{}",
                repo.full_name,
                destination_branch
            );

            match self
                .octocrab
                .post::<serde_json::Value, serde_json::Value>(endpoint, Some(&body))
                .await
            {
                Ok(response) => {
                    tracing::debug!("merge-upstream response: {response}");
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to sync fork {fork_repo} with upstream {}/{destination_branch}: {e:#}",
                        repo.full_name
                    );
                    return Err(e).with_context(|| {
                        format!(
                            "Failed to sync fork {fork_repo} with upstream {}/{destination_branch}",
                            repo.full_name
                        )
                    });
                }
            }

            tracing::info!(
                "Successfully synced fork {fork_repo} with upstream destination branch '{destination_branch}'"
            );
        }

        Ok(fork_repo)
    }

    /// Configure token-based auth on a bare clone via URL rewrite in git config.
    async fn configure_token_auth(&self, bare_dir: &std::path::Path) -> anyhow::Result<()> {
        let token = &self.backend_config.github_token;
        git(
            bare_dir,
            &[
                "config",
                &format!("url.https://x-access-token:{token}@github.com/.insteadOf"),
                "https://github.com/",
            ],
        )
        .await
    }

    /// Ensure a bare clone exists at `repos_dir/{repo_name}.git` with token auth configured.
    async fn ensure_bare_clone_github(
        &self,
        repo: &GitHubRepo,
    ) -> anyhow::Result<PathBuf> {
        let bare_dir = self
            .backend_config
            .repos_dir
            .join(format!("{}.git", repo.name()));

        fs::create_dir_all(&self.backend_config.repos_dir).await?;

        if !bare_dir.exists() {
            let token = &self.backend_config.github_token;
            let clone_url = format!(
                "https://x-access-token:{token}@github.com/{}.git",
                repo.full_name
            );
            let bare_name = format!("{}.git", repo.name());
            tracing::info!("Creating bare clone of {} at {}", repo.full_name, bare_dir.display());
            git(
                &self.backend_config.repos_dir,
                &["clone", "--bare", &clone_url, &bare_name],
            )
            .await?;

            // Configure URL rewrite for subsequent operations
            self.configure_token_auth(&bare_dir).await?;

            // Normalize origin URL to remove embedded token
            let clean_url = format!("https://github.com/{}.git", repo.full_name);
            git(
                &bare_dir,
                &["config", "remote.origin.url", &clean_url],
            )
            .await?;
        }

        // Configure fetch refspec so worktrees get proper origin/* refs
        git(
            &bare_dir,
            &[
                "config",
                "remote.origin.fetch",
                "+refs/heads/*:refs/remotes/origin/*",
            ],
        )
        .await?;

        tracing::info!("Fetching origin in {}", bare_dir.display());
        git(&bare_dir, &["fetch", "origin"]).await?;

        Ok(bare_dir)
    }

    /// Set up fork remote on bare clone for cross-org mode.
    /// Returns `(push_remote_name, pr_repo_full_name)`.
    async fn ensure_fork_remote(
        &self,
        bare_dir: &std::path::Path,
        target_repo: &str,
        base_branch: &str,
    ) -> anyhow::Result<(String, String)> {
        let fork_repo = self.ensure_fork(target_repo, base_branch).await?;

        // Check if "fork" remote exists
        let has_fork = git_check(bare_dir, &["remote", "get-url", "fork"]).await?;
        if !has_fork {
            let fork_url = format!("https://github.com/{fork_repo}.git");
            tracing::info!("Adding fork remote: {fork_url}");
            git(bare_dir, &["remote", "add", "fork", &fork_url]).await?;
        }

        git(bare_dir, &["fetch", "fork"]).await?;

        Ok(("fork".to_string(), fork_repo))
    }

    /// Create a worktree at `workspace_path` for `work_branch` from `base_branch`.
    async fn ensure_worktree_github(
        &self,
        bare_dir: &std::path::Path,
        base_branch: &str,
        work_branch: &str,
        workspace_path: &std::path::Path,
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
        if git_check(
            bare_dir,
            &["rev-parse", &format!("{}^{{commit}}", work_branch)],
        )
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

        zbobr_utility::configure_git_user(workspace_path, &self.git_user_name, &self.git_user_email)
            .await?;

        Ok(())
    }

    /// Ensure a PR exists for the work branch. Creates a placeholder commit and
    /// pushes if needed, then creates the PR (or finds existing).
    async fn ensure_pr(
        &self,
        workspace_path: &std::path::Path,
        push_remote: &str,
        pr_repo: &str,
        work_branch: &str,
        base_branch: &str,
    ) -> anyhow::Result<()> {
        // Check if the branch has any commits ahead of the base branch
        let log_out = git_output(
            workspace_path,
            &[
                "log",
                &format!("origin/{}..HEAD", base_branch),
                "--oneline",
            ],
        )
        .await;

        let has_commits_ahead = log_out
            .as_ref()
            .map(|o| !o.trim().is_empty())
            .unwrap_or(false);

        if !has_commits_ahead {
            tracing::info!(
                "No commits ahead of origin/{base_branch} — creating placeholder commit"
            );
            zbobr_utility::create_placeholder_commit(workspace_path, work_branch).await?;
        }

        // Push to remote
        tracing::info!("Pushing {work_branch} to {push_remote}");
        git(
            workspace_path,
            &[
                "push",
                "--force",
                push_remote,
                &format!("HEAD:{work_branch}"),
            ],
        )
        .await?;

        // Create PR
        let pr_payload = serde_json::json!({
            "title": work_branch,
            "head": work_branch,
            "base": base_branch,
            "body": "",
        });

        #[derive(serde::Deserialize)]
        struct PrResponse {
            #[allow(dead_code)]
            html_url: String,
        }

        let pr_endpoint = format!("/repos/{pr_repo}/pulls");

        let create_result: Result<PrResponse, octocrab::Error> = self
            .octocrab
            .post(pr_endpoint.clone(), Some(&pr_payload))
            .await;

        match create_result {
            Ok(_pr) => {
                tracing::info!("Created PR for {work_branch} in {pr_repo}");
            }
            Err(octocrab::Error::GitHub { ref source, .. })
                if source.status_code.as_u16() == 422 =>
            {
                tracing::info!("PR already exists for {work_branch} in {pr_repo}");
            }
            Err(e) => return Err(octocrab_to_anyhow(e)),
        }

        Ok(())
    }

    /// Find the worktree path for a given work branch by scanning bare clones.
    async fn find_worktree_for_branch(
        &self,
        work_branch: &str,
    ) -> anyhow::Result<(PathBuf, PathBuf)> {
        if !self.backend_config.repos_dir.exists() {
            anyhow::bail!("No worktree found for work_branch '{}'", work_branch);
        }

        let mut entries = fs::read_dir(&self.backend_config.repos_dir)
            .await
            .context("Failed to read repos_dir")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir()
                || !path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().ends_with(".git"))
            {
                continue;
            }

            let output = match git_output(&path, &["worktree", "list", "--porcelain"]).await {
                Ok(o) => o,
                Err(_) => continue,
            };

            for block in output.split("\n\n") {
                let mut wt_path = None;
                let mut branch = None;
                for line in block.lines() {
                    if let Some(p) = line.strip_prefix("worktree ") {
                        wt_path = Some(PathBuf::from(p));
                    }
                    if let Some(b) = line.strip_prefix("branch refs/heads/") {
                        branch = Some(b.to_string());
                    }
                }
                if branch.as_deref() == Some(work_branch)
                    && let Some(wt) = wt_path {
                        return Ok((path.clone(), wt));
                    }
            }
        }

        anyhow::bail!("No worktree found for work_branch '{}'", work_branch)
    }

    /// Find an existing PR URL for a work branch by querying the GitHub API.
    async fn find_pr_for_branch(
        &self,
        bare_dir: &std::path::Path,
        work_branch: &str,
    ) -> anyhow::Result<String> {
        // Determine push remote and derive pr_repo
        let has_fork = git_check(bare_dir, &["remote", "get-url", "fork"]).await?;
        let push_remote = if has_fork { "fork" } else { "origin" };

        let remote_url = git_output(bare_dir, &["remote", "get-url", push_remote]).await?;
        let pr_repo_ref = parse_github_repo(&remote_url)?;

        // Query for open PR with this head branch
        #[derive(serde::Deserialize)]
        struct PrListItem {
            html_url: String,
        }

        let owner = pr_repo_ref.owner();
        let head_filter = format!("{owner}:{work_branch}");
        let endpoint = format!("/repos/{}/pulls", pr_repo_ref.full_name);
        let params = serde_json::json!({
            "head": head_filter,
            "state": "open",
        });

        let prs: Vec<PrListItem> = self
            .octocrab
            .get(&endpoint, Some(&params))
            .await
            .map_err(octocrab_to_anyhow)?;

        prs.into_iter()
            .next()
            .map(|pr| pr.html_url)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No existing open PR found for head '{}' in {}",
                    work_branch,
                    pr_repo_ref.full_name
                )
            })
    }

    /// Query GitHub for an existing open PR matching `head` → `base`.
    /// Returns the `html_url` of the first matching PR.
    async fn find_existing_pr(
        &self,
        full_repo: &str,
        head: &str,
        base: &str,
    ) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct PrListItem {
            html_url: String,
        }

        // GitHub's list-PRs API requires "owner:branch" format for the head
        // filter; a bare branch name is silently ignored and all open PRs are
        // returned, causing the wrong PR to be selected.
        let owner = full_repo.split('/').next().unwrap_or(full_repo);
        let head_filter = format!("{owner}:{head}");

        let endpoint = format!("/repos/{full_repo}/pulls");
        let params = serde_json::json!({
            "head": head_filter,
            "base": base,
            "state": "open",
        });

        let prs: Vec<PrListItem> = self
            .octocrab
            .get(&endpoint, Some(&params))
            .await
            .map_err(octocrab_to_anyhow)?;

        prs.into_iter().next().map(|pr| pr.html_url).ok_or_else(|| {
            anyhow::anyhow!(
                "No existing open PR found for head '{}' -> base '{}' in {}",
                head,
                base,
                full_repo
            )
        })
    }
}

#[async_trait]
impl RepoBackend for ZbobrRepoBackendGithub {
    async fn clone_and_setup(
        &self,
        target_repo: &str,
        work_branch: &str,
        destination_branch: &str,
        workspace_path: &std::path::Path,
    ) -> anyhow::Result<PathBuf> {
        let repo = parse_github_repo(target_repo)?;
        let repo_name = repo.name();

        let work_dir = workspace_path.join(repo_name);

        tokio::fs::create_dir_all(workspace_path).await?;

        // Clone or update the destination repo
        if !work_dir.exists() {
            // Fresh clone: always clone the destination branch (not the work branch,
            // which may not exist on origin yet).
            tracing::info!(
                "Cloning {target_repo} (branch '{destination_branch}') into {}",
                work_dir.display()
            );
            let status = tokio::process::Command::new("gh")
                .args([
                    "repo",
                    "clone",
                    target_repo,
                    work_dir.to_str().unwrap(),
                    "--",
                    "--branch",
                    destination_branch,
                    "--single-branch",
                    "--depth",
                    "1",
                ])
                .env("GH_TOKEN", &self.backend_config.github_token)
                .env("GITHUB_TOKEN", &self.backend_config.github_token)
                .status()
                .await?;
            if !status.success() {
                anyhow::bail!(
                    "Failed to clone {target_repo} at destination branch '{destination_branch}'"
                );
            }
        } else {
            // Workspace exists: fetch the destination branch from origin and
            // force-reset the local branch to match it exactly.
            tracing::info!("Updating {target_repo} in {}", work_dir.display());
            let fetch_output = tokio::process::Command::new("git")
                .args(["fetch", "--depth", "1", "origin", destination_branch])
                .current_dir(&work_dir)
                .output()
                .await?;
            if !fetch_output.status.success() {
                let stderr = String::from_utf8_lossy(&fetch_output.stderr);
                anyhow::bail!(
                    "Failed to fetch destination branch '{destination_branch}' from \
                     {target_repo}: {stderr}"
                );
            }

            // Force-reset the local destination branch so it is identical to origin.
            // Use `git branch -f` so we don't have to switch branches first.
            let reset_output = tokio::process::Command::new("git")
                .args([
                    "branch",
                    "-f",
                    destination_branch,
                    &format!("origin/{destination_branch}"),
                ])
                .current_dir(&work_dir)
                .output()
                .await?;
            if !reset_output.status.success() {
                let stderr = String::from_utf8_lossy(&reset_output.stderr);
                anyhow::bail!(
                    "Failed to reset local branch '{destination_branch}' to \
                     origin/{destination_branch}: {stderr}"
                );
            }
            tracing::info!(
                "Reset local '{destination_branch}' to match origin/{destination_branch}"
            );
        }

        // Determine same-org vs cross-org mode and set up fork remote BEFORE
        // checking out the work branch — in cross-org mode the work branch
        // lives on the fork remote and must be fetched from there.
        let same_org = repo
            .owner()
            .eq_ignore_ascii_case(&self.backend_config.fork_owner);
        let push_remote = if same_org {
            tracing::info!("Same-org mode: skipping fork setup for {target_repo}");
            "origin"
        } else {
            let fork_repo = self.ensure_fork(target_repo, destination_branch).await?;

            // Add fork remote if not present
            let remote_check = tokio::process::Command::new("git")
                .args(["remote", "get-url", "fork"])
                .current_dir(&work_dir)
                .output()
                .await?;

            if !remote_check.status.success() {
                tracing::info!("Adding fork remote for {fork_repo}");
                let status = tokio::process::Command::new("git")
                    .args([
                        "remote",
                        "add",
                        "fork",
                        &format!("https://github.com/{fork_repo}.git"),
                    ])
                    .current_dir(&work_dir)
                    .status()
                    .await?;
                if !status.success() {
                    anyhow::bail!("Failed to add fork remote");
                }
            }

            "fork"
        };

        // Fetch the work branch from the push remote so we can recover
        // previous commits even when the local workspace is fresh.
        // Without this, a fresh clone only has the destination branch and
        // the work branch would be created from scratch, losing all history.
        if work_branch != destination_branch {
            tracing::info!("Fetching work branch '{work_branch}' from {push_remote}");
            let fetch_work = tokio::process::Command::new("git")
                .args(["fetch", push_remote, work_branch])
                .current_dir(&work_dir)
                .output()
                .await?;
            if fetch_work.status.success() {
                tracing::info!(
                    "Fetched work branch '{work_branch}' from {push_remote}"
                );
            } else {
                tracing::info!(
                    "Work branch '{work_branch}' not found on {push_remote} (may be new)"
                );
            }
        }

        // Clean up any broken rebase/merge state left by a previous session
        // before attempting to checkout. Without this, checkout will fail with
        // "you need to resolve your current index first".
        let rebase_merge_dir = work_dir.join(".git/rebase-merge");
        let rebase_apply_dir = work_dir.join(".git/rebase-apply");
        if rebase_merge_dir.exists() || rebase_apply_dir.exists() {
            tracing::warn!("Detected in-progress rebase in {}, aborting", work_dir.display());
            let _ = tokio::process::Command::new("git")
                .args(["rebase", "--abort"])
                .current_dir(&work_dir)
                .status()
                .await;
        }
        let merge_head = work_dir.join(".git/MERGE_HEAD");
        if merge_head.exists() {
            tracing::warn!("Detected in-progress merge in {}, aborting", work_dir.display());
            let _ = tokio::process::Command::new("git")
                .args(["merge", "--abort"])
                .current_dir(&work_dir)
                .status()
                .await;
        }

        // Checkout the work branch
        tracing::info!("Checking out branch {work_branch}");
        let checkout_status = tokio::process::Command::new("git")
            .args(["checkout", work_branch])
            .current_dir(&work_dir)
            .status()
            .await?;
        if !checkout_status.success() {
            // Try from the push remote (fork in cross-org, origin in same-org)
            let checkout_remote_status = tokio::process::Command::new("git")
                .args([
                    "checkout",
                    "-b",
                    work_branch,
                    &format!("{push_remote}/{work_branch}"),
                ])
                .current_dir(&work_dir)
                .status()
                .await?;
            if !checkout_remote_status.success() {
                // In cross-org mode, also try origin as a fallback
                let fallback_ok = if push_remote != "origin" {
                    tokio::process::Command::new("git")
                        .args([
                            "checkout",
                            "-b",
                            work_branch,
                            &format!("origin/{work_branch}"),
                        ])
                        .current_dir(&work_dir)
                        .status()
                        .await?
                        .success()
                } else {
                    false
                };
                if !fallback_ok {
                    // Branch doesn't exist on any remote — create it fresh from the destination branch.
                    tracing::info!(
                        "Creating new local branch {work_branch} from {destination_branch}"
                    );
                    let create_status = tokio::process::Command::new("git")
                        .args(["checkout", "-b", work_branch])
                        .current_dir(&work_dir)
                        .status()
                        .await?;
                    if !create_status.success() {
                        anyhow::bail!("Failed to checkout branch {work_branch}");
                    }
                }
            }
        } else {
            // Local branch existed — check if the remote has newer commits
            // (e.g. from a previous session on a different machine) and
            // fast-forward to include them so we don't overwrite history.
            let remote_ref = format!("{push_remote}/{work_branch}");
            let has_remote = tokio::process::Command::new("git")
                .args(["rev-parse", "--verify", &remote_ref])
                .current_dir(&work_dir)
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if has_remote {
                // Check if local HEAD is an ancestor of the remote ref.
                // If so, fast-forward to include the remote commits.
                let is_ancestor = tokio::process::Command::new("git")
                    .args(["merge-base", "--is-ancestor", "HEAD", &remote_ref])
                    .current_dir(&work_dir)
                    .output()
                    .await
                    .map(|o| o.status.success())
                    .unwrap_or(false);

                if is_ancestor {
                    tracing::info!(
                        "Fast-forwarding local '{work_branch}' to {remote_ref}"
                    );
                    let _ = tokio::process::Command::new("git")
                        .args(["merge", "--ff-only", &remote_ref])
                        .current_dir(&work_dir)
                        .status()
                        .await;
                } else {
                    tracing::info!(
                        "Local '{work_branch}' has commits not on {remote_ref}, keeping local state"
                    );
                }
            }
        }

        Ok(work_dir)
    }

    async fn clone_readonly(
        &self,
        target_repo: &str,
        branch: &str,
        workspace_path: &std::path::Path,
    ) -> anyhow::Result<PathBuf> {
        let repo = parse_github_repo(target_repo)?;
        let repo_name = repo.name();

        let work_dir = workspace_path.join(repo_name);

        tokio::fs::create_dir_all(workspace_path).await?;

        if !work_dir.exists() {
            tracing::info!(
                "Cloning {target_repo} (read-only) into {},",
                work_dir.display()
            );
            let status = tokio::process::Command::new("gh")
                .args([
                    "repo",
                    "clone",
                    target_repo,
                    work_dir.to_str().unwrap(),
                    "--",
                    "--branch",
                    branch,
                    "--single-branch",
                    "--depth",
                    "1",
                ])
                .env("GH_TOKEN", &self.backend_config.github_token)
                .env("GITHUB_TOKEN", &self.backend_config.github_token)
                .status()
                .await?;
            if !status.success() {
                anyhow::bail!("Failed to clone {target_repo}");
            }
        } else {
            tracing::info!(
                "Updating {target_repo} (read-only) in {}",
                work_dir.display()
            );

            let fetch_status = tokio::process::Command::new("git")
                .args(["fetch", "--depth", "1", "origin", branch])
                .current_dir(&work_dir)
                .status()
                .await?;

            if !fetch_status.success() {
                tracing::warn!(
                    "Failed to fetch latest changes for {target_repo}, using existing state"
                );
            }
        }

        // Checkout the requested branch
        tracing::info!("Checking out branch {branch} (read-only)");
        let checkout_status = tokio::process::Command::new("git")
            .args(["checkout", branch])
            .current_dir(&work_dir)
            .status()
            .await?;
        if !checkout_status.success() {
            let checkout_remote_status = tokio::process::Command::new("git")
                .args(["checkout", "-b", branch, &format!("origin/{branch}")])
                .current_dir(&work_dir)
                .status()
                .await?;
            if !checkout_remote_status.success() {
                anyhow::bail!("Failed to checkout branch {branch}");
            }
        }

        Ok(work_dir)
    }

    async fn ensure_branch_and_pr(
        &self,
        target_repo: &str,
        workspace_path: &std::path::Path,
        work_branch: &str,
        destination_branch: &str,
        pr_title: &str,
    ) -> anyhow::Result<String> {
        let repo = parse_github_repo(target_repo)?;
        let work_dir = workspace_path.join(repo.name());

        if !work_dir.exists() {
            anyhow::bail!("Work directory does not exist: {}", work_dir.display());
        }

        // Check if the branch has any commits ahead of the destination branch.
        // If not, create a placeholder commit so GitHub PR API won't reject it.
        let log_out = tokio::process::Command::new("git")
            .args([
                "log",
                &format!("origin/{}..HEAD", destination_branch),
                "--oneline",
            ])
            .current_dir(&work_dir)
            .output()
            .await
            .context("Failed to check commits ahead of destination branch")?;

        let has_commits_ahead =
            log_out.status.success() && !String::from_utf8_lossy(&log_out.stdout).trim().is_empty();

        if !has_commits_ahead {
            tracing::info!(
                "No commits ahead of origin/{destination_branch} — creating placeholder commit"
            );
            zbobr_utility::configure_git_user(&work_dir, &self.git_user_name, &self.git_user_email)
                .await
                .context("Failed to configure git user for placeholder commit")?;
            zbobr_utility::create_placeholder_commit(&work_dir, work_branch)
                .await
                .context("Failed to create placeholder commit")?;
        }

        // In same-org mode there is no "fork" remote — push directly to origin
        // and use a simple branch name for the PR head.  In cross-org mode push
        // to the fork remote and prefix the head with the fork owner.
        let has_fork_remote = tokio::process::Command::new("git")
            .args(["remote", "get-url", "fork"])
            .current_dir(&work_dir)
            .output()
            .await?
            .status
            .success();

        // In cross-org mode push to the fork remote and create the PR inside
        // the fork.  The user is responsible for retargeting the PR to the
        // upstream repo.  In same-org mode push directly to origin and create
        // the PR there.
        let (push_remote, pr_repo) = if has_fork_remote {
            (
                "fork",
                format!("{}/{}", self.backend_config.fork_owner, repo.name()),
            )
        } else {
            ("origin", repo.full_name.clone())
        };

        tracing::info!("Pushing {work_branch} to {push_remote}");
        let status = tokio::process::Command::new("git")
            // .args(["push", "--force", push_remote, "HEAD"])
            .args(["push", push_remote, "HEAD"])
            .current_dir(&work_dir)
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("Failed to push to {push_remote}");
        }

        // Try to create the PR.  If GitHub returns 422 (PR already exists for this branch),
        // query the open PRs to find and return the existing PR URL.
        let pr_payload = serde_json::json!({
            "title": pr_title,
            "head": work_branch,
            "base": destination_branch,
            "body": "",
        });

        #[derive(serde::Deserialize)]
        struct PrResponse {
            html_url: String,
        }

        let pr_endpoint = format!("/repos/{pr_repo}/pulls");

        let create_result: Result<PrResponse, octocrab::Error> = self
            .octocrab
            .post(pr_endpoint.clone(), Some(&pr_payload))
            .await;

        match create_result {
            Ok(pr) => Ok(pr.html_url),
            Err(octocrab::Error::GitHub { ref source, .. })
                if source.status_code.as_u16() == 422 =>
            {
                tracing::info!("PR already exists for {work_branch}, looking up existing PR");
                self.find_existing_pr(&pr_repo, work_branch, destination_branch)
                    .await
            }
            Err(e) => Err(octocrab_to_anyhow(e)),
        }
    }

    async fn push_branch(
        &self,
        target_repo: &str,
        workspace_path: &std::path::Path,
        work_branch: &str,
    ) -> anyhow::Result<()> {
        let repo = parse_github_repo(target_repo)?;
        let work_dir = workspace_path.join(repo.name());

        if !work_dir.exists() {
            anyhow::bail!("Work directory does not exist: {}", work_dir.display());
        }

        let has_fork_remote = tokio::process::Command::new("git")
            .args(["remote", "get-url", "fork"])
            .current_dir(&work_dir)
            .output()
            .await?
            .status
            .success();

        let push_remote = if has_fork_remote { "fork" } else { "origin" };

        tracing::info!("Pushing {work_branch} to {push_remote}");
        let status = tokio::process::Command::new("git")
            // .args(["push", "--force", push_remote, "HEAD"])
            .args(["push", push_remote, "HEAD"])
            .current_dir(&work_dir)
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("Failed to push to {push_remote}");
        }

        Ok(())
    }

    async fn create_pr_in_fork(
        &self,
        repo_name: &str,
        work_branch: &str,
        destination_branch: &str,
        pr_title: &str,
        pr_body: &str,
    ) -> anyhow::Result<String> {
        let fork_repo = format!("{}/{}", self.backend_config.fork_owner, repo_name);

        tracing::info!(
            "Creating PR in fork {} from {} to {}",
            fork_repo,
            work_branch,
            destination_branch
        );

        let pr_payload = serde_json::json!({
            "title": pr_title,
            "head": work_branch,
            "base": destination_branch,
            "body": pr_body,
        });

        let pr_endpoint = format!("/repos/{fork_repo}/pulls");

        #[derive(serde::Deserialize)]
        struct PrResponse {
            html_url: String,
        }

        let response: PrResponse = retry_github("create PR in fork", || {
            self.octocrab.post(pr_endpoint.clone(), Some(&pr_payload))
        })
        .await?;

        Ok(response.html_url)
    }

    async fn setup_fork_remote_and_push(
        &self,
        work_dir: &std::path::Path,
        target_repo: &str,
        work_branch: &str,
    ) -> anyhow::Result<()> {
        let repo = parse_github_repo(target_repo)?;
        let fork_repo = format!("{}/{}", self.backend_config.fork_owner, repo.name());
        let fork_url = format!("https://github.com/{fork_repo}.git");

        // Remove old "fork" remote (ignore error if it doesn't exist)
        let _ = tokio::process::Command::new("git")
            .args(["remote", "remove", "fork"])
            .current_dir(work_dir)
            .status()
            .await;

        // Remove origin remote and replace it with fork remote URL
        tracing::info!("Replacing origin remote with fork: {}", fork_url);
        let remove_origin = tokio::process::Command::new("git")
            .args(["remote", "remove", "origin"])
            .current_dir(work_dir)
            .status()
            .await?;

        if !remove_origin.success() {
            anyhow::bail!("Failed to remove origin remote");
        }

        let add_origin = tokio::process::Command::new("git")
            .args(["remote", "add", "origin", &fork_url])
            .current_dir(work_dir)
            .status()
            .await?;

        if !add_origin.success() {
            anyhow::bail!("Failed to add fork as origin remote");
        }

        // Push the work branch to the forked repository
        tracing::info!("Pushing work branch '{}' to fork", work_branch);
        let push_status = tokio::process::Command::new("git")
            .args(["push", "-u", "origin", work_branch])
            .current_dir(work_dir)
            .status()
            .await?;

        if !push_status.success() {
            anyhow::bail!("Failed to push work branch '{}' to fork", work_branch);
        }

        Ok(())
    }

    async fn parse_pr_to_repo_branch(&self, pr_ref: &str) -> anyhow::Result<(String, String)> {
        fn parse_ref(pr_ref: &str) -> anyhow::Result<(String, String, u64)> {
            if pr_ref.starts_with("https://github.com/") {
                let parts: Vec<&str> = pr_ref
                    .trim_start_matches("https://github.com/")
                    .split('/')
                    .collect();
                if parts.len() >= 4 && parts[2] == "pull" {
                    let pr_num = parts[3]
                        .parse::<u64>()
                        .map_err(|_| anyhow::anyhow!("Invalid PR number in URL: {pr_ref}"))?;
                    return Ok((parts[0].to_string(), parts[1].to_string(), pr_num));
                }
                return Err(anyhow::anyhow!("Invalid PR URL format: {pr_ref}"));
            }
            if pr_ref.contains('#') {
                let parts: Vec<&str> = pr_ref.split('#').collect();
                if parts.len() == 2 {
                    let repo_parts: Vec<&str> = parts[0].split('/').collect();
                    if repo_parts.len() == 2 {
                        let pr_num = parts[1]
                            .parse::<u64>()
                            .map_err(|_| anyhow::anyhow!("Invalid PR number: {}", parts[1]))?;
                        return Ok((repo_parts[0].to_string(), repo_parts[1].to_string(), pr_num));
                    }
                    return Err(anyhow::anyhow!(
                        "Invalid repo format in PR reference: {pr_ref}"
                    ));
                }
                return Err(anyhow::anyhow!("Invalid PR reference format: {pr_ref}"));
            }
            Err(anyhow::anyhow!(
                "PR reference must be a URL or owner/repo#number format: {pr_ref}"
            ))
        }

        let (owner, repo, pr_number) = parse_ref(pr_ref)?;

        #[derive(serde::Deserialize)]
        struct PrHead {
            #[serde(rename = "ref")]
            ref_name: String,
        }
        #[derive(serde::Deserialize)]
        struct PrView {
            head: PrHead,
        }

        let pr_endpoint = format!("/repos/{owner}/{repo}/pulls/{pr_number}");
        let pr: PrView = self
            .octocrab
            .get(pr_endpoint, None::<&()>)
            .await
            .map_err(|e| anyhow::anyhow!("GitHub API error: {}", e))?;

        let branch = pr.head.ref_name;
        let repo_full = format!("{owner}/{repo}");

        Ok((repo_full, branch))
    }

    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        let fork_owner = &self.backend_config.fork_owner;
        let fork_owner_exists = retry_github("check fork owner", || {
            self.octocrab
                .get::<serde_json::Value, _, _>(format!("/users/{fork_owner}"), None::<&()>)
        })
        .await
        .is_ok();
        if !fork_owner_exists {
            anyhow::bail!(
                "fork_owner '{fork_owner}' does not exist on GitHub as a user or organization.\n  \
                 Check your fork_owner setting and ensure the account exists."
            );
        }

        Ok(())
    }

    fn debug_state(&self) -> String {
        format!(
            "GitHubRepoBackend(fork_owner={})",
            self.backend_config.fork_owner
        )
    }
}

#[async_trait]
impl zbobr_api::backend::WorktreeBackend for ZbobrRepoBackendGithub {
    async fn update_worktree(
        &self,
        remote_repo: &str,
        base_branch: &str,
        work_branch: &str,
        workspace_path: &std::path::Path,
    ) -> anyhow::Result<bool> {
        if work_branch == base_branch {
            anyhow::bail!(
                "work_branch and base_branch must differ, got '{}'",
                work_branch
            );
        }

        let repo = parse_github_repo(remote_repo)?;
        let bare_dir = self.ensure_bare_clone_github(&repo).await?;

        // Determine same-org vs cross-org
        let same_org = repo
            .owner()
            .eq_ignore_ascii_case(&self.backend_config.fork_owner);

        let (push_remote, pr_repo) = if same_org {
            tracing::info!("Same-org mode: skipping fork setup for {}", repo.full_name);
            ("origin".to_string(), repo.full_name.clone())
        } else {
            self.ensure_fork_remote(&bare_dir, remote_repo, base_branch)
                .await?
        };

        self.ensure_worktree_github(&bare_dir, base_branch, work_branch, workspace_path)
            .await?;

        // Create PR early so it exists before update_pr is called
        self.ensure_pr(workspace_path, &push_remote, &pr_repo, work_branch, base_branch)
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
        let (bare_dir, worktree_path) = self.find_worktree_for_branch(work_branch).await?;

        // Determine push remote
        let has_fork = git_check(&bare_dir, &["remote", "get-url", "fork"]).await?;
        let push_remote = if has_fork { "fork" } else { "origin" };

        // Push from worktree
        tracing::info!("Pushing {work_branch} to {push_remote}");
        git(
            &worktree_path,
            &[
                "push",
                "--force",
                push_remote,
                &format!("HEAD:{work_branch}"),
            ],
        )
        .await?;

        // Find and return PR URL
        self.find_pr_for_branch(&bare_dir, work_branch).await
    }

    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        // Check fork owner exists
        let fork_owner = &self.backend_config.fork_owner;
        let fork_owner_exists = retry_github("check fork owner", || {
            self.octocrab
                .get::<serde_json::Value, _, _>(format!("/users/{fork_owner}"), None::<&()>)
        })
        .await
        .is_ok();
        if !fork_owner_exists {
            anyhow::bail!(
                "fork_owner '{fork_owner}' does not exist on GitHub as a user or organization.\n  \
                 Check your fork_owner setting and ensure the account exists."
            );
        }

        // Verify repos_dir is writable
        fs::create_dir_all(&self.backend_config.repos_dir)
            .await
            .with_context(|| {
                format!(
                    "Cannot access repos directory '{}'",
                    self.backend_config.repos_dir.display()
                )
            })?;

        let test_path = self.backend_config.repos_dir.join(".test");
        fs::write(&test_path, "test").await.with_context(|| {
            format!(
                "Cannot write to repos directory '{}'",
                self.backend_config.repos_dir.display()
            )
        })?;
        let _ = fs::remove_file(&test_path).await;

        Ok(())
    }

    fn debug_state(&self) -> String {
        format!(
            "GitHubRepoBackend(fork_owner={}, repos_dir={})",
            self.backend_config.fork_owner,
            self.backend_config.repos_dir.display()
        )
    }
}
