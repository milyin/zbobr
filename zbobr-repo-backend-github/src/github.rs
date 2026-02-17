use std::{path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;

use zbobr_dispatcher::backend::RepoBackend;
use zbobr_dispatcher::{ZbobrDispatcherConfig, ZbobrError};

use crate::config::ZbobrRepoBackendGithubConfig;

/// Convert an octocrab error into a ZbobrError with detailed information.
fn octocrab_to_zbobr_error(e: octocrab::Error) -> ZbobrError {
    let error_msg = match e {
        octocrab::Error::GitHub { source, .. } => {
            format!(
                "GitHub API error: {} (status: {}) -- details: {:?}",
                source.message, source.status_code, source
            )
        }
        other => format!("GitHub API error: {:?}", other),
    };
    ZbobrError::GitHub(error_msg)
}

fn is_transient_octocrab_error(error: &octocrab::Error) -> bool {
    match error {
        octocrab::Error::GitHub { source, .. } => source.status_code.is_server_error(),
        _ => true,
    }
}

/// Generates a `retry` method on a struct that has an `octocrab` field.
/// The method retries transient GitHub API errors up to 3 times.
/// Closures capture `self.octocrab` from the surrounding scope (zero-arg).
macro_rules! impl_retry {
    ($type:ty) => {
        impl $type {
            async fn retry<T, F, Fut>(&self, op_name: &str, mut f: F) -> Result<T, ZbobrError>
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
                            return Err(octocrab_to_zbobr_error(e));
                        }
                    }
                }
            }
        }
    };
}

impl_retry!(GitHubRepoBackend);

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct RepoResponse {
    full_name: String,
}

// ============================================================================
// GitHubRepoBackend
// ============================================================================

pub struct GitHubRepoBackend {
    config: Arc<ZbobrDispatcherConfig>,
    backend_config: ZbobrRepoBackendGithubConfig,
    octocrab: octocrab::Octocrab,
}

impl GitHubRepoBackend {
    pub fn new(
        config: Arc<ZbobrDispatcherConfig>,
        toml: Option<&crate::config::ZbobrRepoBackendGithubToml>,
        fork_owner_override: Option<&str>,
    ) -> Result<Self, ZbobrError> {
        let backend_config = ZbobrRepoBackendGithubConfig::build(toml, fork_owner_override);
        backend_config.validate()?;
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(backend_config.github_token.clone())
            .build()
            .map_err(|e| ZbobrError::GitHub(format!("Failed to build octocrab client: {e}")))?;
        Ok(Self { config, backend_config, octocrab })
    }

    async fn ensure_fork(&self, target_repo: &str) -> Result<String, ZbobrError> {
        let repo_name = target_repo
            .split('/')
            .nth(1)
            .ok_or_else(|| ZbobrError::Config(format!("Invalid repo format: {target_repo}")))?;

        let fork_repo = format!("{}/{}", self.backend_config.fork_owner, repo_name);

        // Check if fork already exists
        let exists = self.retry("check fork exists", || {
            self.octocrab.get::<RepoResponse, _, _>(format!("/repos/{fork_repo}"), None::<&()>)
        })
        .await
        .is_ok();

        if !exists {
            let parts: Vec<&str> = target_repo.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(ZbobrError::Config(format!(
                    "Invalid target repo: {target_repo}"
                )));
            }
            let fork_owner = &self.backend_config.fork_owner;
            let endpoint = format!("/repos/{}/{}/forks", parts[0], parts[1]);
            let payload = serde_json::json!({ "organization": fork_owner });

            tracing::info!("Creating fork of {target_repo} under organization '{fork_owner}' using endpoint {endpoint}");
            tracing::debug!("Fork creation payload: {payload}");

            self.retry("create fork", || {
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
                ZbobrError::GitHub(
                    format!(
                        "Failed to create fork of {target_repo} under '{fork_owner}': \
                         check if fork_owner is an organization you have access to, \
                         and that your GitHub token has 'repo' and 'admin:org_hook' scopes. \
                         Endpoint: {endpoint}. Error: {e}\n\
                         Debug: {error_details}",
                    )
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
        task_id: u64,
    ) -> Result<PathBuf, ZbobrError> {
        let repo_name = target_repo
            .split('/')
            .nth(1)
            .ok_or_else(|| ZbobrError::Config(format!("Invalid repo format: {target_repo}")))?;

        let task_dir = self.config.workspace.join(format!("task#{task_id}"));
        let work_dir = task_dir.join(repo_name);

        tokio::fs::create_dir_all(&task_dir).await?;

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
                return Err(ZbobrError::Other(format!("Failed to clone {target_repo}")));
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
                return Err(ZbobrError::Other(format!(
                    "Failed to checkout branch {branch}"
                )));
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
                return Err(ZbobrError::Other("Failed to add fork remote".into()));
            }
        }

        Ok(work_dir)
    }

    async fn clone_readonly(
        &self,
        target_repo: &str,
        branch: &str,
        task_id: u64,
    ) -> Result<PathBuf, ZbobrError> {
        let repo_name = target_repo
            .split('/')
            .nth(1)
            .ok_or_else(|| ZbobrError::Config(format!("Invalid repo format: {target_repo}")))?;

        let task_dir = self.config.workspace.join(format!("task#{task_id}"));
        let work_dir = task_dir.join(repo_name);

        tokio::fs::create_dir_all(&task_dir).await?;

        if !work_dir.exists() {
            tracing::info!(
                "Cloning {target_repo} (read-only) into {}",
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
                return Err(ZbobrError::Other(format!("Failed to clone {target_repo}")));
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
                return Err(ZbobrError::Other(format!(
                    "Failed to checkout branch {branch}"
                )));
            }
        }

        Ok(work_dir)
    }

    async fn push_and_create_pr(
        &self,
        target_repo: &str,
        task_id: u64,
        pr_title: &str,
        pr_body: &str,
    ) -> Result<String, ZbobrError> {
        let repo_name = target_repo
            .split('/')
            .nth(1)
            .ok_or_else(|| ZbobrError::Config(format!("Invalid repo format: {target_repo}")))?;

        let work_dir = self
            .config
            .workspace
            .join(format!("task#{task_id}"))
            .join(repo_name);

        if !work_dir.exists() {
            return Err(ZbobrError::Other(format!(
                "Work directory does not exist: {}",
                work_dir.display()
            )));
        }

        let branch_name = {
            let out = tokio::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&work_dir)
                .output()
                .await
                .map_err(|e| ZbobrError::Other(format!("Failed to determine current branch: {}", e)))?;
            if !out.status.success() {
                return Err(ZbobrError::Other("Failed to determine current branch".to_string()));
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
                        tracing::info!("Local changes detected; removing placeholder {} and committing changes", branch_name);
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
                        let commit_msg = format!("chore: remove placeholder {} and apply changes", &branch_name);
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
            return Err(ZbobrError::Other("Failed to push to fork".into()));
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

        let pr_endpoint = format!("/repos/{target_repo}/pulls");
        let response: PrResponse = self
            .octocrab
            .post(pr_endpoint, Some(&pr_payload))
            .await
            .map_err(|e| ZbobrError::GitHub(e.to_string()))?;

        Ok(response.html_url)
    }

    async fn create_pr_in_fork(
        &self,
        repo_name: &str,
        work_branch: &str,
        destination_branch: &str,
        pr_title: &str,
        pr_body: &str,
    ) -> Result<String, ZbobrError> {
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

        let response: PrResponse = self.retry("create PR", || {
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
    ) -> Result<(), ZbobrError> {
        let repo_name = target_repo
            .split('/')
            .nth(1)
            .ok_or_else(|| ZbobrError::Other(format!("Invalid target_repo format: {}", target_repo)))?;
        let fork_repo = format!("{}/{}", self.backend_config.fork_owner, repo_name);
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
            return Err(ZbobrError::Other("Failed to remove origin remote".to_string()));
        }

        let add_origin = tokio::process::Command::new("git")
            .args(["remote", "add", "origin", &fork_url])
            .current_dir(work_dir)
            .status()
            .await?;

        if !add_origin.success() {
            return Err(ZbobrError::Other("Failed to add fork as origin remote".to_string()));
        }

        // Push the work branch to the forked repository
        tracing::info!("Pushing work branch '{}' to fork", work_branch);
        let push_status = tokio::process::Command::new("git")
            .args(["push", "-u", "origin", work_branch])
            .current_dir(work_dir)
            .status()
            .await?;

        if !push_status.success() {
            return Err(ZbobrError::Other(format!(
                "Failed to push work branch '{}' to fork",
                work_branch
            )));
        }

        Ok(())
    }

    async fn sync_fork(&self, target_repo: &str, branch: &str) -> Result<(), ZbobrError> {
        let parts: Vec<&str> = target_repo.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(ZbobrError::Config(format!("Invalid target repo: {}", target_repo)));
        }
        let upstream_owner = parts[0];

        let fork_repo = self.ensure_fork(target_repo).await?;

        let endpoint = format!("/repos/{}/merge-upstream", fork_repo);
        let body = serde_json::json!({
            "branch": branch,
            "upstream": format!("{}:{}", upstream_owner, branch),
            "commit_message": format!("Sync fork {} from {}/{}", fork_repo, upstream_owner, branch),
        });

        tracing::info!("Calling merge-upstream for {} -> {}", fork_repo, branch);

        match self.octocrab.post::<serde_json::Value, serde_json::Value>(endpoint, Some(&body)).await {
            Ok(_) => {
                tracing::info!("Successfully synced fork {} from {}/{}", fork_repo, upstream_owner, branch);
                Ok(())
            }
            Err(e) => {
                tracing::error!("merge-upstream failed for {}: {}", fork_repo, e);
                Err(octocrab_to_zbobr_error(e))
            }
        }
    }

    async fn parse_pr_to_repo_branch(&self, pr_ref: &str) -> Result<(String, String), ZbobrError> {
        let (owner, repo, pr_number) = if pr_ref.starts_with("https://github.com/") {
            let parts: Vec<&str> = pr_ref
                .trim_start_matches("https://github.com/")
                .split('/')
                .collect();
            if parts.len() >= 4 && parts[2] == "pull" {
                let owner = parts[0];
                let repo = parts[1];
                let pr_num = parts[3].parse::<u64>().map_err(|_| {
                    ZbobrError::Other(format!("Invalid PR number in URL: {pr_ref}"))
                })?;
                (owner.to_string(), repo.to_string(), pr_num)
            } else {
                return Err(ZbobrError::Other(format!(
                    "Invalid PR URL format: {pr_ref}"
                )));
            }
        } else if pr_ref.contains('#') {
            let parts: Vec<&str> = pr_ref.split('#').collect();
            if parts.len() == 2 {
                let repo_parts: Vec<&str> = parts[0].split('/').collect();
                if repo_parts.len() == 2 {
                    let owner = repo_parts[0];
                    let repo = repo_parts[1];
                    let pr_num = parts[1].parse::<u64>().map_err(|_| {
                        ZbobrError::Other(format!("Invalid PR number: {}", parts[1]))
                    })?;
                    (owner.to_string(), repo.to_string(), pr_num)
                } else {
                    return Err(ZbobrError::Other(format!(
                        "Invalid repo format in PR reference: {pr_ref}"
                    )));
                }
            } else {
                return Err(ZbobrError::Other(format!(
                    "Invalid PR reference format: {pr_ref}"
                )));
            }
        } else {
            return Err(ZbobrError::Other(format!(
                "PR reference must be a URL or owner/repo#number format: {pr_ref}"
            )));
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
            .map_err(|e| ZbobrError::GitHub(e.to_string()))?;

        let branch = pr.head.ref_name;
        let repo_full = format!("{owner}/{repo}");

        Ok((repo_full, branch))
    }

    async fn validate_connectivity(&self) -> Result<(), ZbobrError> {
        let fork_owner = &self.backend_config.fork_owner;
        let fork_owner_exists = self.retry("check fork owner", || {
            self.octocrab.get::<serde_json::Value, _, _>(format!("/users/{fork_owner}"), None::<&()>)
        })
        .await
        .is_ok();
        if !fork_owner_exists {
            return Err(ZbobrError::Config(format!(
                "fork_owner '{fork_owner}' does not exist on GitHub as a user or organization.\n  \
                 Check your fork_owner setting and ensure the account exists."
            )));
        }

        Ok(())
    }

    fn debug_state(&self) -> String {
        format!("GitHubRepoBackend(fork_owner={})", self.backend_config.fork_owner)
    }
}
