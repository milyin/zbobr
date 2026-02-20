use std::{path::PathBuf, time::Duration};

use anyhow::Context;
use async_trait::async_trait;

use zbobr_dispatcher::backend::RepoBackend;

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
// GitHubRepoBackend
// ============================================================================

pub struct GitHubRepoBackend {
    backend_config: ZbobrRepoBackendGithubConfig,
    octocrab: octocrab::Octocrab,
}

impl GitHubRepoBackend {
    pub fn new(
        toml: Option<crate::config::ZbobrRepoBackendGithubToml>,
        args: crate::config::ZbobrRepoBackendGithubArgs,
    ) -> anyhow::Result<Self> {
        let backend_config = ZbobrRepoBackendGithubConfig::build(toml, args);
        backend_config.validate()?;
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(backend_config.github_token.clone())
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build octocrab client: {e}"))?;
        Ok(Self {
            backend_config,
            octocrab,
        })
    }

    async fn ensure_fork(&self, target_repo: &str) -> anyhow::Result<String> {
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
            let repo = parse_github_repo(target_repo)?;
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
        }

        Ok(fork_repo)
    }
}

#[async_trait]
impl RepoBackend for GitHubRepoBackend {
    async fn clone_and_setup(
        &self,
        target_repo: &str,
        branch: &str,
        workspace_path: &std::path::Path,
    ) -> anyhow::Result<PathBuf> {
        let repo = parse_github_repo(target_repo)?;
        let repo_name = repo.name();

        let work_dir = workspace_path.join(repo_name);

        tokio::fs::create_dir_all(workspace_path).await?;

        // Clone if not already present
        if !work_dir.exists() {
            tracing::info!("Cloning {target_repo} into {}", work_dir.display());
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
            tracing::info!("Updating {target_repo} in {}", work_dir.display());
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
        tracing::info!("Checking out branch {branch}");
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

        // Ensure fork exists
        let fork_repo = self.ensure_fork(target_repo).await?;

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

    async fn push_and_create_pr(
        &self,
        target_repo: &str,
        workspace_path: &std::path::Path,
        pr_title: &str,
        pr_body: &str,
    ) -> anyhow::Result<String> {
        let repo = parse_github_repo(target_repo)?;
        let work_dir = workspace_path.join(repo.name());

        if !work_dir.exists() {
            anyhow::bail!("Work directory does not exist: {}", work_dir.display());
        }

        let branch_name = {
            let out = tokio::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&work_dir)
                .output()
                .await
                .context("Failed to determine current branch")?;
            if !out.status.success() {
                anyhow::bail!("Failed to determine current branch");
            }
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };

        // Remove placeholder if present
        let zbobr_placeholder = work_dir.join(".zbobr").join(&branch_name);
        if zbobr_placeholder.exists() {
            match tokio::process::Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(&work_dir)
                .output()
                .await
            {
                Ok(out) => {
                    if !out.stdout.is_empty() {
                        tracing::info!(
                            "Local changes detected; removing placeholder {} and committing changes",
                            branch_name
                        );
                        let _ = tokio::process::Command::new("git")
                            .args(["rm", "-f", format!(".zbobr/{}", &branch_name).as_str()])
                            .current_dir(&work_dir)
                            .status()
                            .await;
                        let _ = tokio::process::Command::new("git")
                            .args(["add", "-A"])
                            .current_dir(&work_dir)
                            .status()
                            .await;
                        let commit_msg = format!(
                            "chore: remove placeholder {} and apply changes",
                            &branch_name
                        );
                        let commit_status = tokio::process::Command::new("git")
                            .args(["commit", "-m", &commit_msg])
                            .current_dir(&work_dir)
                            .status()
                            .await;
                        if let Err(e) = commit_status {
                            tracing::warn!("Failed to commit after removing placeholder: {}", e);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to check git status: {}", e),
            }
        }

        tracing::info!("Pushing {branch_name} to fork");
        let status = tokio::process::Command::new("git")
            .args(["push", "fork", "HEAD"])
            .current_dir(&work_dir)
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("Failed to push to fork");
        }

        // Create PR
        let pr_payload = serde_json::json!({
            "title": pr_title,
            "head": format!("{}:{branch_name}", self.backend_config.fork_owner),
            "body": pr_body,
        });

        #[derive(serde::Deserialize)]
        struct PrResponse {
            html_url: String,
        }

        let pr_endpoint = format!("/repos/{}/pulls", repo.full_name);
        let response: PrResponse = self
            .octocrab
            .post(pr_endpoint, Some(&pr_payload))
            .await
            .map_err(|e| anyhow::anyhow!("GitHub API error: {}", e))?;

        Ok(response.html_url)
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
            "Creating PR in {} from {} to {} using octocrab",
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

        let response: PrResponse = retry_github("create PR", || {
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

    async fn sync_fork(&self, target_repo: &str, branch: &str) -> anyhow::Result<()> {
        let repo = parse_github_repo(target_repo)?;
        let fork_repo = self.ensure_fork(target_repo).await?;

        let endpoint = format!("/repos/{}/merge-upstream", fork_repo);
        let body = serde_json::json!({
            "branch": branch,
            "upstream": format!("{}:{}", repo.owner(), branch),
            "commit_message": format!("Sync fork {} from {}/{}", fork_repo, repo.owner(), branch),
        });

        tracing::info!("Calling merge-upstream for {} -> {}", fork_repo, branch);

        match self
            .octocrab
            .post::<serde_json::Value, serde_json::Value>(endpoint, Some(&body))
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "Successfully synced fork {} from {}/{}",
                    fork_repo,
                    repo.owner(),
                    branch
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!("merge-upstream failed for {}: {}", fork_repo, e);
                Err(octocrab_to_anyhow(e))
            }
        }
    }

    async fn parse_pr_to_repo_branch(&self, pr_ref: &str) -> anyhow::Result<(String, String)> {
        let (owner, repo, pr_number) = if pr_ref.starts_with("https://github.com/") {
            let parts: Vec<&str> = pr_ref
                .trim_start_matches("https://github.com/")
                .split('/')
                .collect();
            if parts.len() >= 4 && parts[2] == "pull" {
                let owner = parts[0];
                let repo = parts[1];
                let pr_num = parts[3]
                    .parse::<u64>()
                    .map_err(|_| anyhow::anyhow!("Invalid PR number in URL: {pr_ref}"))?;
                (owner.to_string(), repo.to_string(), pr_num)
            } else {
                anyhow::bail!("Invalid PR URL format: {pr_ref}");
            }
        } else if pr_ref.contains('#') {
            let parts: Vec<&str> = pr_ref.split('#').collect();
            if parts.len() == 2 {
                let repo_parts: Vec<&str> = parts[0].split('/').collect();
                if repo_parts.len() == 2 {
                    let owner = repo_parts[0];
                    let repo = repo_parts[1];
                    let pr_num = parts[1]
                        .parse::<u64>()
                        .map_err(|_| anyhow::anyhow!("Invalid PR number: {}", parts[1]))?;
                    (owner.to_string(), repo.to_string(), pr_num)
                } else {
                    anyhow::bail!("Invalid repo format in PR reference: {pr_ref}");
                }
            } else {
                anyhow::bail!("Invalid PR reference format: {pr_ref}");
            }
        } else {
            anyhow::bail!("PR reference must be a URL or owner/repo#number format: {pr_ref}");
        };

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
