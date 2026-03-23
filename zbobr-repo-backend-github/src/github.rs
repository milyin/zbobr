use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use tokio::fs;
use zbobr_api::backend::WorktreeBackend;
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

/// Extract the backing bare repository path from a worktree's `.git` file.
///
/// A worktree's `.git` is a file containing `gitdir: /path/to/bare.git/worktrees/<name>`.
/// This function reads that file and walks up two parents to find the bare clone directory.
async fn worktree_bare_dir(workspace_path: &Path) -> Option<PathBuf> {
    let git_file = workspace_path.join(".git");
    if !git_file.is_file() {
        return None;
    }
    let content = tokio::fs::read_to_string(&git_file).await.ok()?;
    let gitdir = content.strip_prefix("gitdir: ")?.trim();
    // gitdir points to <bare>.git/worktrees/<name>, walk up twice to get <bare>.git
    let bare = Path::new(gitdir).parent()?.parent()?;
    Some(bare.to_path_buf())
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

    /// Returns a filesystem-safe name for the bare clone directory: "owner__repo"
    fn bare_dir_name(&self) -> String {
        format!("{}__{}", self.owner(), self.name())
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

#[derive(Clone)]
pub struct ZbobrRepoBackendGithub {
    backend_config: ZbobrRepoBackendGithubConfig,
    octocrab: octocrab::Octocrab,
}

impl ZbobrRepoBackendGithub {
    pub fn from_config(backend_config: ZbobrRepoBackendGithubConfig) -> anyhow::Result<Self> {
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

    /// Create from config and validate connectivity to GitHub.
    pub async fn new(backend_config: ZbobrRepoBackendGithubConfig) -> anyhow::Result<Self> {
        let backend = Self::from_config(backend_config)?;
        backend.validate_connectivity().await?;
        Ok(backend)
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
    async fn configure_token_auth(&self, bare_dir: &Path) -> anyhow::Result<()> {
        // Remove any existing insteadOf entries for github.com to avoid stale tokens
        // accumulating as separate config keys (the token is part of the key).
        if let Ok(output) = git_output(
            bare_dir,
            &["config", "--get-regexp", r"url\..*github\.com.*\.insteadOf"],
        )
        .await
        {
            for line in output.lines() {
                if let Some(key) = line.split_whitespace().next() {
                    let _ = git(bare_dir, &["config", "--unset", key]).await;
                }
            }
        }

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

    /// Ensure a bare clone exists at `repos_dir/{owner}__{repo_name}.git` with token auth configured.
    async fn ensure_bare_clone_github(&self, repo: &GitHubRepo) -> anyhow::Result<PathBuf> {
        let bare_dir = self
            .backend_config
            .repos_dir
            .join(format!("{}.git", repo.bare_dir_name()));

        fs::create_dir_all(&self.backend_config.repos_dir).await?;

        if !bare_dir.exists() {
            let token = &self.backend_config.github_token;
            let clone_url = format!(
                "https://x-access-token:{token}@github.com/{}.git",
                repo.full_name
            );
            let bare_name = format!("{}.git", repo.bare_dir_name());
            tracing::info!(
                "Creating bare clone of {} at {}",
                repo.full_name,
                bare_dir.display()
            );
            git(
                &self.backend_config.repos_dir,
                &["clone", "--bare", &clone_url, &bare_name],
            )
            .await?;

            // Normalize origin URL to remove embedded token
            let clean_url = format!("https://github.com/{}.git", repo.full_name);
            git(&bare_dir, &["config", "remote.origin.url", &clean_url]).await?;
        }

        // Configure URL rewrite for subsequent operations (unconditionally,
        // so token auth is refreshed even when the bare clone already exists)
        self.configure_token_auth(&bare_dir).await?;

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
        bare_dir: &Path,
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
        bare_dir: &Path,
        base_branch: &str,
        work_branch: &str,
        workspace_path: &Path,
        git_user_name: &str,
        git_user_email: &str,
    ) -> anyhow::Result<()> {
        if workspace_path.exists() {
            // Verify the worktree is linked to the expected bare clone
            let linked_bare = worktree_bare_dir(workspace_path).await;
            let bare_canon = bare_dir.canonicalize().ok();
            let linked_canon = linked_bare.as_ref().and_then(|p| p.canonicalize().ok());

            if bare_canon.is_some() && bare_canon == linked_canon {
                tracing::info!(
                    "Worktree already exists at {} and is linked to correct bare clone",
                    workspace_path.display()
                );
                return Ok(());
            }

            // Stale worktree: linked to a different (old) bare clone
            tracing::warn!(
                "Worktree at {} is linked to {:?}, expected {}. Removing stale worktree.",
                workspace_path.display(),
                linked_bare
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "unknown".into()),
                bare_dir.display(),
            );

            // Clean up the old bare clone's worktree tracking (best-effort)
            if let Some(ref old_bare) = linked_bare {
                let _ = git(old_bare, &["worktree", "prune"]).await;
            }

            // Remove the stale worktree directory
            if let Err(e) = fs::remove_dir_all(workspace_path).await {
                tracing::error!(
                    "Failed to remove stale worktree at {}: {}",
                    workspace_path.display(),
                    e
                );
                anyhow::bail!(
                    "Cannot remove stale worktree at {}: {}",
                    workspace_path.display(),
                    e
                );
            }

            tracing::info!("Removed stale worktree, will recreate from correct bare clone");
            // Fall through to create a new worktree from the correct bare_dir
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

        zbobr_utility::configure_git_user(workspace_path, git_user_name, git_user_email).await?;

        Ok(())
    }

    /// Ensure a PR exists on GitHub for the given work branch (API-only, no push).
    /// Creates a draft PR or silently succeeds if one already exists (422).
    async fn ensure_pr_exists(
        &self,
        pr_repo: &str,
        work_branch: &str,
        base_branch: &str,
    ) -> anyhow::Result<()> {
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

    /// Cross-org only: sync the fork's base branch with upstream via merge-upstream API.
    /// Fetches both remotes afterwards so local refs are current.
    async fn sync_fork_base_with_upstream(
        &self,
        bare_dir: &Path,
        base_branch: &str,
        fork_repo: &str,
    ) -> anyhow::Result<()> {
        // Check if origin/{base} and fork/{base} point to the same commit
        let origin_ref = format!("origin/{base_branch}");
        let fork_ref = format!("fork/{base_branch}");

        let origin_sha = git_output(bare_dir, &["rev-parse", &origin_ref]).await;
        let fork_sha = git_output(bare_dir, &["rev-parse", &fork_ref]).await;

        let needs_sync = match (&origin_sha, &fork_sha) {
            (Ok(a), Ok(b)) => a.trim() != b.trim(),
            _ => true, // If either ref is missing, sync anyway
        };

        if !needs_sync {
            tracing::info!("Fork base branch '{base_branch}' is already in sync with upstream");
            return Ok(());
        }

        tracing::info!("Syncing fork {fork_repo} base branch '{base_branch}' with upstream");

        let endpoint = format!("/repos/{fork_repo}/merge-upstream");
        let body = serde_json::json!({ "branch": base_branch });

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
                    "Failed to sync fork {fork_repo} base branch '{base_branch}' with upstream: {e:#}"
                );
                return Err(octocrab_to_anyhow(e));
            }
        }

        // Re-fetch fork so local refs are updated
        git(bare_dir, &["fetch", "fork"]).await?;

        Ok(())
    }

    /// Ensure local `refs/heads/{base_branch}` matches `refs/remotes/{remote}/{base_branch}`.
    /// Uses `git update-ref` to force-set the local ref (safe — bare repo local base is just a copy).
    async fn sync_local_base_ref(
        bare_dir: &Path,
        base_branch: &str,
        remote: &str,
    ) -> anyhow::Result<()> {
        let remote_ref = format!("refs/remotes/{remote}/{base_branch}");
        let local_ref = format!("refs/heads/{base_branch}");

        let remote_sha = git_output(bare_dir, &["rev-parse", &remote_ref]).await;

        match remote_sha {
            Ok(remote_sha) => {
                let local_sha = git_output(bare_dir, &["rev-parse", &local_ref]).await;

                let needs_update = match &local_sha {
                    Ok(sha) => sha.trim() != remote_sha.trim(),
                    Err(_) => true,
                };

                if needs_update {
                    tracing::info!("Updating local {local_ref} to match {remote_ref}");
                    git(bare_dir, &["update-ref", &local_ref, remote_sha.trim()]).await?;
                }
            }
            Err(_) => {
                let local_exists =
                    git_check(bare_dir, &["rev-parse", "--verify", &local_ref]).await?;
                if local_exists {
                    tracing::warn!(
                        "Remote ref {remote_ref} not found, using existing local {local_ref}"
                    );
                } else {
                    anyhow::bail!(
                        "Neither remote ref {remote_ref} nor local ref {local_ref} exist"
                    );
                }
            }
        }

        Ok(())
    }

    /// Fetch `{push_remote}/{work_branch}` with explicit refspec.
    /// Returns `false` if the remote branch doesn't exist yet.
    async fn fetch_remote_work_branch(
        bare_dir: &Path,
        push_remote: &str,
        work_branch: &str,
    ) -> anyhow::Result<bool> {
        let refspec = format!("refs/heads/{work_branch}:refs/remotes/{push_remote}/{work_branch}");

        let ok = git_check(bare_dir, &["fetch", push_remote, &refspec]).await?;

        if ok {
            tracing::info!("Fetched {push_remote}/{work_branch}");
        } else {
            tracing::info!("Remote branch {push_remote}/{work_branch} does not exist yet");
        }

        Ok(ok)
    }

    /// Stash any uncommitted changes (including untracked files) in the worktree.
    /// Returns whether a stash was created.
    async fn stash_worktree_changes(worktree_path: &Path) -> anyhow::Result<bool> {
        let status = git_output(worktree_path, &["status", "--porcelain"]).await?;
        if status.trim().is_empty() {
            return Ok(false);
        }

        tracing::info!("Stashing uncommitted changes in worktree");
        git(
            worktree_path,
            &[
                "stash",
                "push",
                "--include-untracked",
                "-m",
                "auto-stash before merge",
            ],
        )
        .await?;
        Ok(true)
    }

    /// Merge `source_ref` into the current HEAD in the worktree.
    /// Skips if `source_ref` is already an ancestor of HEAD.
    /// Returns `true` on success, `false` on conflict (leaves mid-merge state).
    async fn merge_ref_into_worktree(
        worktree_path: &Path,
        source_ref: &str,
    ) -> anyhow::Result<bool> {
        // Check if already merged
        let already_merged = git_check(
            worktree_path,
            &["merge-base", "--is-ancestor", source_ref, "HEAD"],
        )
        .await?;

        if already_merged {
            tracing::info!("{source_ref} is already merged into HEAD");
            return Ok(true);
        }

        tracing::info!("Merging {source_ref} into worktree HEAD");
        let ok = git_check(worktree_path, &["merge", source_ref, "--no-edit"]).await?;

        if ok {
            tracing::info!("Successfully merged {source_ref}");
        } else {
            tracing::warn!("Merge conflict while merging {source_ref}");
        }

        Ok(ok)
    }

    /// Push worktree HEAD to remote without --force.
    /// Errors if remote has diverged (requires merge first).
    async fn push_worktree_to_remote(
        worktree_path: &Path,
        push_remote: &str,
        work_branch: &str,
    ) -> anyhow::Result<()> {
        tracing::info!("Pushing {work_branch} to {push_remote} (no force)");
        git(
            worktree_path,
            &["push", push_remote, &format!("HEAD:{work_branch}")],
        )
        .await
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
                    && let Some(wt) = wt_path
                {
                    return Ok((path.clone(), wt));
                }
            }
        }

        anyhow::bail!("No worktree found for work_branch '{}'", work_branch)
    }

    /// Query GitHub for an existing open PR matching `head` → `base`.
    /// Returns the `html_url` of the first matching PR.
    async fn find_existing_pr(
        &self,
        full_repo: &str,
        head: &str,
        base: Option<&str>,
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
        let mut params = serde_json::json!({
            "head": head_filter,
            "state": "open",
        });
        if let Some(base) = base {
            params["base"] = serde_json::Value::String(base.to_string());
        }

        let prs: Vec<PrListItem> = self
            .octocrab
            .get(&endpoint, Some(&params))
            .await
            .map_err(octocrab_to_anyhow)?;

        prs.into_iter().next().map(|pr| pr.html_url).ok_or_else(|| {
            anyhow::anyhow!(
                "No existing open PR found for head '{}' -> base '{:?}' in {}",
                head,
                base,
                full_repo
            )
        })
    }
}

#[async_trait]
impl zbobr_api::backend::WorktreeBackend for ZbobrRepoBackendGithub {
    /// Merge-based update_worktree flow. Never force-pushes the work branch.
    ///
    /// ## Algorithm
    ///
    /// Phase 1 – Setup: parse repo, ensure bare clone (fetches origin),
    ///   determine same-org vs cross-org, ensure fork remote if cross-org.
    ///
    /// Phase 2 – Validate base branch sync:
    ///   cross-org: sync fork base with upstream via merge-upstream API.
    ///   All: sync local refs/heads/{base_branch} to match remote.
    ///
    /// Phase 3 – Fetch remote work branch (may not exist yet).
    ///
    /// Phase 4 – Create worktree (reuse ensure_worktree_github).
    ///
    /// Phase 5 – Ensure PR exists:
    ///   If remote work branch doesn't exist: create placeholder commit, regular push, create PR.
    ///   If remote work branch exists: just ensure_pr_exists (API only).
    ///
    /// Phase 6 – Abort any in-progress merge from a previous failed run.
    ///
    /// Phase 7 – Stash uncommitted changes in worktree.
    ///
    /// Phase 8 – Merge remote work → local work (element 5 → 6):
    ///   skip if remote doesn't exist. On conflict → return Ok(false).
    ///
    /// Phase 9 – Merge base → local work (element 4 → 6):
    ///   on conflict → return Ok(false).
    ///
    /// Phase 10 – Push result back (no --force). Return Ok(true).
    async fn update_worktree(
        &self,
        identity: &zbobr_api::task::TaskIdentity,
        workspace_path: &Path,
        git_user_name: &str,
        git_user_email: &str,
    ) -> anyhow::Result<bool> {
        let remote_repo = &identity.destination_repository;
        let base_branch = &identity.destination_branch;
        let work_branch = &identity.work_branch;

        if work_branch == base_branch {
            anyhow::bail!(
                "work_branch and base_branch must differ, got '{}'",
                work_branch
            );
        }

        // Phase 1: Setup
        let repo = parse_github_repo(remote_repo)?;
        let bare_dir = self.ensure_bare_clone_github(&repo).await?;

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

        // Phase 2: Validate base branch sync
        let base_remote = if same_org { "origin" } else { "fork" };
        if !same_org {
            self.sync_fork_base_with_upstream(&bare_dir, base_branch, &pr_repo)
                .await?;
        }
        Self::sync_local_base_ref(&bare_dir, base_branch, base_remote).await?;

        // Phase 3: Fetch remote work branch
        let remote_exists =
            Self::fetch_remote_work_branch(&bare_dir, &push_remote, work_branch).await?;

        // Phase 4: Create worktree
        self.ensure_worktree_github(
            &bare_dir,
            base_branch,
            work_branch,
            workspace_path,
            git_user_name,
            git_user_email,
        )
        .await?;

        // Phase 5: Ensure PR exists
        if !remote_exists {
            // Need to push first so the PR can be created
            tracing::info!(
                "Remote work branch does not exist — creating placeholder commit and pushing"
            );

            // Check if worktree has any commits ahead of base
            let log_out = git_output(
                workspace_path,
                &["log", &format!("{}..HEAD", base_branch), "--oneline"],
            )
            .await;
            let has_commits_ahead = log_out
                .as_ref()
                .map(|o| !o.trim().is_empty())
                .unwrap_or(false);

            if !has_commits_ahead {
                zbobr_utility::create_placeholder_commit(workspace_path, work_branch).await?;
            }

            // Regular push (not force) — branch is new, so no conflict possible
            Self::push_worktree_to_remote(workspace_path, &push_remote, work_branch).await?;

            // Now create the PR
            self.ensure_pr_exists(&pr_repo, work_branch, base_branch)
                .await?;
        } else {
            self.ensure_pr_exists(&pr_repo, work_branch, base_branch)
                .await?;
        }

        // Phase 6: Abort any in-progress merge from a previous failed run
        let merge_head = workspace_path.join(".git/MERGE_HEAD");
        // For worktrees, .git is a file pointing to the bare repo, so check the
        // gitdir path for MERGE_HEAD as well.
        let gitdir_merge_head = {
            let git_file = workspace_path.join(".git");
            if git_file.is_file() {
                // .git file contains "gitdir: /path/to/bare/.git/worktrees/..."
                if let Ok(content) = tokio::fs::read_to_string(&git_file).await {
                    content
                        .strip_prefix("gitdir: ")
                        .map(|p| PathBuf::from(p.trim()).join("MERGE_HEAD"))
                } else {
                    None
                }
            } else {
                None
            }
        };

        let has_merge_in_progress =
            merge_head.exists() || gitdir_merge_head.as_ref().is_some_and(|p| p.exists());

        if has_merge_in_progress {
            tracing::warn!("Detected in-progress merge in worktree, aborting before proceeding");
            let _ = git_check(workspace_path, &["merge", "--abort"]).await;
        }

        // Phase 7: Stash uncommitted changes
        Self::stash_worktree_changes(workspace_path).await?;

        // Phase 8: Merge remote work → local work (element 5 → 6)
        if remote_exists {
            let remote_ref = format!("{push_remote}/{work_branch}");
            let merged = Self::merge_ref_into_worktree(workspace_path, &remote_ref).await?;
            if !merged {
                tracing::warn!("Merge conflict merging remote work branch — needs merger");
                return Ok(false);
            }
        }

        // Phase 9: Merge base → local work (element 4 → 6)
        let merged = Self::merge_ref_into_worktree(workspace_path, base_branch).await?;
        if !merged {
            tracing::warn!("Merge conflict merging base branch — needs merger");
            return Ok(false);
        }

        // Phase 10: Push result back (no --force)
        Self::push_worktree_to_remote(workspace_path, &push_remote, work_branch).await?;

        tracing::info!("Worktree {work_branch}: up-to-date, all merges succeeded, pushed");
        Ok(true)
    }

    async fn update_pr(&self, identity: &zbobr_api::task::TaskIdentity) -> anyhow::Result<String> {
        let work_branch = &identity.work_branch;
        let destination_repo = &identity.destination_repository;
        let base_branch = &identity.destination_branch;
        // 1. Find the worktree and bare_dir for this branch
        let (_bare_dir, worktree_path) = self.find_worktree_for_branch(work_branch).await?;

        // 2. Stash any uncommitted changes
        Self::stash_worktree_changes(&worktree_path).await?;

        // 3. Determine push remote and PR repo
        let repo = parse_github_repo(destination_repo)?;
        let same_org = repo
            .owner()
            .eq_ignore_ascii_case(&self.backend_config.fork_owner);
        let (push_remote, pr_repo) = if same_org {
            ("origin", repo.full_name.clone())
        } else {
            (
                "fork",
                format!("{}/{}", self.backend_config.fork_owner, repo.name()),
            )
        };

        // 4. Push to remote (no --force)
        Self::push_worktree_to_remote(&worktree_path, push_remote, work_branch).await?;

        // 5. Find existing PR or create a new one
        if let Ok(url) = self.find_existing_pr(&pr_repo, work_branch, None).await {
            return Ok(url);
        }

        // No existing PR — create one
        tracing::info!("No existing PR found for {work_branch}, creating one in {pr_repo}");

        #[derive(serde::Deserialize)]
        struct PrResponse {
            html_url: String,
        }

        let endpoint = format!("/repos/{pr_repo}/pulls");
        let pr_payload = serde_json::json!({
            "title": work_branch,
            "head": work_branch,
            "base": base_branch,
            "body": "",
        });

        let create_result: Result<PrResponse, octocrab::Error> =
            self.octocrab.post(&endpoint, Some(&pr_payload)).await;

        match create_result {
            Ok(pr) => Ok(pr.html_url),
            Err(octocrab::Error::GitHub { ref source, .. })
                if source.status_code.as_u16() == 422 =>
            {
                // Race condition: PR was created between our check and create
                tracing::info!("PR already exists (422), looking up existing PR");
                self.find_existing_pr(&pr_repo, work_branch, Some(base_branch))
                    .await
            }
            Err(e) => Err(octocrab_to_anyhow(e)),
        }
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
