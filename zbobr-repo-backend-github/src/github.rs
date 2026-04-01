use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use tokio::fs;
use zbobr_api::backend::WorktreeBackend;
use zbobr_utility::{git, git_check, git_check_env, git_env, git_output};

use crate::config::ZbobrRepoBackendGithubConfig;

struct ExistingPr {
    html_url: String,
    number: u64,
}

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
    pub fn from_config(mut backend_config: ZbobrRepoBackendGithubConfig) -> anyhow::Result<Self> {
        backend_config.validate()?;
        // Normalize repository to "owner/repo" format so all downstream API calls work
        // regardless of whether the user supplied an HTTPS URL, SSH URL, or bare "owner/repo".
        let repo = parse_github_repo(&backend_config.repository)?;
        backend_config.repository = repo.full_name;
        let token = backend_config.github_token.as_ref().to_owned();
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(token)
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

    /// Build environment variables that configure git token auth via
    /// `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_*` / `GIT_CONFIG_VALUE_*`.
    /// Uses `http.extraheader` with a base64-encoded Authorization header so
    /// the token never appears in URLs, command-line args, or on-disk config.
    fn token_auth_env(&self) -> anyhow::Result<[(&str, String); 3]> {
        use base64::Engine as _;
        let token = self.backend_config.github_token.as_ref();
        let credentials =
            base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
        Ok([
            ("GIT_CONFIG_COUNT", "1".into()),
            (
                "GIT_CONFIG_KEY_0",
                "http.https://github.com/.extraheader".into(),
            ),
            (
                "GIT_CONFIG_VALUE_0",
                format!("Authorization: basic {credentials}"),
            ),
        ])
    }

    /// Remove legacy insteadOf entries that embedded the token in git config.
    async fn cleanup_legacy_token_config(&self, bare_dir: &Path) -> anyhow::Result<()> {
        let output = match git_output(
            bare_dir,
            &["config", "--get-regexp", r"url\..*github\.com.*\.insteadOf"],
        )
        .await
        {
            Ok(output) => output,
            Err(_) => return Ok(()), // No matching entries found
        };

        for line in output.lines() {
            if let Some(key) = line.split_whitespace().next() {
                // Use tokio::process::Command directly instead of git() helper to
                // avoid leaking the token through the error message (git() embeds
                // the full args, which include the legacy key containing the token).
                let status = tokio::process::Command::new("git")
                    .args(["config", "--unset", key])
                    .current_dir(bare_dir)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .await;
                let success = status.as_ref().map_or(false, |s| s.success());
                if !success {
                    // Redact any embedded credentials from the key before logging
                    let redacted_key = if let Some(proto_end) = key.find("://") {
                        if let Some(at_pos) = key[proto_end..].find('@') {
                            format!(
                                "{}://[REDACTED]{}",
                                &key[..proto_end],
                                &key[proto_end + at_pos..]
                            )
                        } else {
                            key.to_string()
                        }
                    } else {
                        key.to_string()
                    };
                    let exit_info = match status {
                        Ok(s) => format!("exit code {}", s.code().unwrap_or(-1)),
                        Err(e) => format!("spawn error: {e}"),
                    };
                    tracing::warn!(
                        "Failed to remove legacy token config key '{}' in {}: {}",
                        redacted_key,
                        bare_dir.display(),
                        exit_info
                    );
                }
            }
        }
        Ok(())
    }

    /// Ensure a bare clone exists at `repos_dir/{owner}__{repo_name}.git` with token auth configured.
    async fn ensure_bare_clone_github(&self, repo: &GitHubRepo) -> anyhow::Result<PathBuf> {
        let bare_dir = self
            .backend_config
            .repos_dir
            .join(format!("{}.git", repo.bare_dir_name()));

        fs::create_dir_all(&self.backend_config.repos_dir).await?;

        let owned_env = self.token_auth_env()?;
        let env: Vec<(&str, &str)> = owned_env.iter().map(|(k, v)| (*k, v.as_str())).collect();

        if !bare_dir.exists() {
            let clone_url = format!("https://github.com/{}.git", repo.full_name);
            let bare_name = format!("{}.git", repo.bare_dir_name());
            tracing::info!(
                "Creating bare clone of {} at {}",
                repo.full_name,
                bare_dir.display()
            );
            git_env(
                &self.backend_config.repos_dir,
                &["clone", "--bare", &clone_url, &bare_name],
                &env,
            )
            .await?;
        }

        // Remove legacy token-in-config entries from existing repos
        self.cleanup_legacy_token_config(&bare_dir).await?;

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
        git_env(&bare_dir, &["fetch", "origin"], &env).await?;

        Ok(bare_dir)
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

    /// Ensure local `refs/heads/{base_branch}` matches `refs/remotes/origin/{base_branch}`.
    /// Uses `git update-ref` to force-set the local ref (safe — bare repo local base is just a copy).
    async fn sync_local_base_ref(bare_dir: &Path, base_branch: &str) -> anyhow::Result<()> {
        let remote_ref = format!("refs/remotes/origin/{base_branch}");
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

    /// Fetch `origin/{work_branch}` with explicit refspec.
    /// Returns `false` if the remote branch doesn't exist yet.
    async fn fetch_remote_work_branch(
        bare_dir: &Path,
        work_branch: &str,
        envs: &[(&str, &str)],
    ) -> anyhow::Result<bool> {
        let refspec =
            format!("refs/heads/{work_branch}:refs/remotes/origin/{work_branch}");

        let ok = git_check_env(bare_dir, &["fetch", "origin", &refspec], envs).await?;

        if ok {
            tracing::info!("Fetched origin/{work_branch}");
        } else {
            tracing::info!("Remote branch origin/{work_branch} does not exist yet");
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

    /// Push worktree HEAD to origin without --force.
    /// Errors if remote has diverged (requires merge first).
    async fn push_worktree_to_origin(
        worktree_path: &Path,
        work_branch: &str,
        envs: &[(&str, &str)],
    ) -> anyhow::Result<()> {
        tracing::info!("Pushing {work_branch} to origin (no force)");
        git_env(
            worktree_path,
            &["push", "origin", &format!("HEAD:{work_branch}")],
            envs,
        )
        .await
    }

    /// Query GitHub for an existing open PR matching `head` → `base`.
    /// Returns the `html_url` of the first matching PR.
    async fn find_existing_pr(
        &self,
        full_repo: &str,
        head: &str,
        base: Option<&str>,
    ) -> anyhow::Result<ExistingPr> {
        #[derive(serde::Deserialize)]
        struct PrListItem {
            html_url: String,
            number: u64,
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

        prs.into_iter()
            .next()
            .map(|pr| ExistingPr {
                html_url: pr.html_url,
                number: pr.number,
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No existing open PR found for head '{}' -> base '{:?}' in {}",
                    head,
                    base,
                    full_repo
                )
            })
    }

    async fn update_pr_body(
        &self,
        pr_repo: &str,
        pr_number: u64,
        body: &str,
    ) -> anyhow::Result<()> {
        let endpoint = format!("/repos/{pr_repo}/pulls/{pr_number}");
        let patch_payload = serde_json::json!({ "body": body });
        self.octocrab
            .patch::<serde_json::Value, _, _>(&endpoint, Some(&patch_payload))
            .await
            .map_err(octocrab_to_anyhow)?;
        Ok(())
    }
}

#[async_trait]
impl zbobr_api::backend::WorktreeBackend for ZbobrRepoBackendGithub {
    fn repository(&self) -> &str {
        &self.backend_config.repository
    }

    fn branch(&self) -> &str {
        &self.backend_config.branch
    }

    fn repo_name(&self) -> &str {
        self.backend_config.repo_short_name()
    }

    /// Merge-based update_worktree flow. Never force-pushes the work branch.
    ///
    /// ## Algorithm
    ///
    /// Phase 1 – Setup: parse repo, ensure bare clone (fetches origin).
    ///
    /// Phase 2 – Validate base branch sync:
    ///   Sync local refs/heads/{base_branch} to match origin.
    ///
    /// Phase 3 – Fetch remote work branch (may not exist yet).
    ///
    /// Phase 4 – Create worktree (reuse ensure_worktree_github).
    ///
    /// Phase 5 – Ensure PR exists:
    ///   If remote work branch doesn't exist: create placeholder commit, regular push, create PR.
    ///   If remote work branch exists: call ensure_pr_url (API only).
    ///
    /// Phase 6 – Abort any in-progress merge from a previous failed run.
    ///
    /// Phase 7 – Stash uncommitted changes in worktree.
    ///
    /// Phase 8 – Merge remote work → local work:
    ///   skip if remote doesn't exist. On conflict → return Ok(false).
    ///
    /// Phase 9 – Merge base → local work:
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
        let base_branch = &self.backend_config.branch;
        let work_branch = &identity.work_branch;

        if work_branch == base_branch {
            anyhow::bail!(
                "work_branch and base_branch must differ, got '{}'",
                work_branch
            );
        }

        let owned_env = self.token_auth_env()?;
        let env: Vec<(&str, &str)> = owned_env.iter().map(|(k, v)| (*k, v.as_str())).collect();

        // Phase 1: Setup
        let repo = parse_github_repo(&self.backend_config.repository)?;
        let bare_dir = self.ensure_bare_clone_github(&repo).await?;

        // Phase 2: Validate base branch sync
        Self::sync_local_base_ref(&bare_dir, base_branch).await?;

        // Phase 3: Fetch remote work branch
        let remote_exists =
            Self::fetch_remote_work_branch(&bare_dir, work_branch, &env).await?;

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

        // Phase 5: Push if new branch
        if !remote_exists {
            // Need to push first so the PR can be created later (by ensure_pr_url)
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
            Self::push_worktree_to_origin(workspace_path, work_branch, &env).await?;
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

        // Phase 8: Merge remote work → local work
        if remote_exists {
            let remote_ref = format!("origin/{work_branch}");
            let merged = Self::merge_ref_into_worktree(workspace_path, &remote_ref).await?;
            if !merged {
                tracing::warn!("Merge conflict merging remote work branch — needs merger");
                return Ok(false);
            }
        }

        // Phase 9: Merge base → local work
        let merged = Self::merge_ref_into_worktree(workspace_path, base_branch).await?;
        if !merged {
            tracing::warn!("Merge conflict merging base branch — needs merger");
            return Ok(false);
        }

        // Phase 10: Push result back (no --force)
        Self::push_worktree_to_origin(workspace_path, work_branch, &env).await?;

        tracing::info!("Worktree {work_branch}: up-to-date, all merges succeeded, pushed");
        Ok(true)
    }

    async fn fetch_refs(&self, _identity: &zbobr_api::task::TaskIdentity) -> anyhow::Result<()> {
        let repo = parse_github_repo(&self.backend_config.repository)?;
        let bare_dir = self.ensure_bare_clone_github(&repo).await?;
        let owned_env = self.token_auth_env()?;
        let env: Vec<(&str, &str)> = owned_env.iter().map(|(k, v)| (*k, v.as_str())).collect();
        git_env(&bare_dir, &["fetch", "origin"], &env).await?;
        Ok(())
    }

    async fn ensure_pr_url(
        &self,
        identity: &zbobr_api::task::TaskIdentity,
        body: Option<&str>,
    ) -> anyhow::Result<String> {
        let work_branch = &identity.work_branch;
        let pr_repo = &self.backend_config.repository;
        let base_branch = &self.backend_config.branch;

        // Find existing PR or create a new one
        if let Ok(existing) = self.find_existing_pr(pr_repo, work_branch, None).await {
            if let Some(body) = body {
                tracing::info!(
                    "Updating body of existing PR #{} for {work_branch}",
                    existing.number
                );
                self.update_pr_body(pr_repo, existing.number, body).await?;
            }
            return Ok(existing.html_url);
        }

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
            "body": body.unwrap_or(""),
        });

        let create_result: Result<PrResponse, octocrab::Error> =
            self.octocrab.post(&endpoint, Some(&pr_payload)).await;

        match create_result {
            Ok(pr) => Ok(pr.html_url),
            Err(octocrab::Error::GitHub { ref source, .. })
                if source.status_code.as_u16() == 422 =>
            {
                tracing::info!("PR already exists (422), looking up existing PR");
                let existing = self
                    .find_existing_pr(pr_repo, work_branch, Some(base_branch))
                    .await?;
                if let Some(body) = body {
                    tracing::info!(
                        "Updating body of existing PR #{} for {work_branch}",
                        existing.number
                    );
                    self.update_pr_body(pr_repo, existing.number, body).await?;
                }
                Ok(existing.html_url)
            }
            Err(e) => Err(octocrab_to_anyhow(e)),
        }
    }

    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        // Check repository exists on GitHub
        let repo_path = &self.backend_config.repository;
        let repo_exists = retry_github("check repository exists", || {
            self.octocrab
                .get::<RepoResponse, _, _>(format!("/repos/{repo_path}"), None::<&()>)
        })
        .await
        .is_ok();
        if !repo_exists {
            anyhow::bail!(
                "repository '{repo_path}' does not exist on GitHub or is not accessible.\n  \
                 Check your repository setting and ensure the token has access."
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
            "GitHubRepoBackend(repository={}, branch={}, repos_dir={})",
            self.backend_config.repository,
            self.backend_config.branch,
            self.backend_config.repos_dir.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbobr_api::Secret;

    // ── parse_github_repo tests ──────────────────────────────────────

    #[test]
    fn parse_owner_repo_plain() {
        let repo = parse_github_repo("owner/repo").unwrap();
        assert_eq!(repo.full_name, "owner/repo");
    }

    #[test]
    fn parse_https_url() {
        let repo = parse_github_repo("https://github.com/owner/repo").unwrap();
        assert_eq!(repo.full_name, "owner/repo");
    }

    #[test]
    fn parse_https_url_with_git_suffix() {
        let repo = parse_github_repo("https://github.com/owner/repo.git").unwrap();
        assert_eq!(repo.full_name, "owner/repo");
    }

    #[test]
    fn parse_https_url_trailing_slash() {
        let repo = parse_github_repo("https://github.com/owner/repo/").unwrap();
        assert_eq!(repo.full_name, "owner/repo");
    }

    #[test]
    fn parse_ssh_url() {
        let repo = parse_github_repo("git@github.com:owner/repo").unwrap();
        assert_eq!(repo.full_name, "owner/repo");
    }

    #[test]
    fn parse_ssh_url_with_git_suffix() {
        let repo = parse_github_repo("git@github.com:owner/repo.git").unwrap();
        assert_eq!(repo.full_name, "owner/repo");
    }

    #[test]
    fn parse_owner_repo_with_git_suffix() {
        let repo = parse_github_repo("owner/repo.git").unwrap();
        assert_eq!(repo.full_name, "owner/repo");
    }

    #[test]
    fn parse_rejects_bare_name() {
        let result = parse_github_repo("just-a-name");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid GitHub repository format"));
    }

    // ── from_config normalization tests ──────────────────────────────

    #[tokio::test]
    async fn from_config_normalizes_https_url() {
        let config = ZbobrRepoBackendGithubConfig {
            repository: "https://github.com/myorg/myrepo.git".to_string(),
            branch: "main".to_string(),
            github_token: Secret::value("ghp_test123"),
            repos_dir: std::path::PathBuf::from("/tmp/test-repos"),
        };
        let backend = ZbobrRepoBackendGithub::from_config(config).unwrap();
        assert_eq!(backend.backend_config.repository, "myorg/myrepo");
    }

    #[tokio::test]
    async fn from_config_normalizes_ssh_url() {
        let config = ZbobrRepoBackendGithubConfig {
            repository: "git@github.com:myorg/myrepo.git".to_string(),
            branch: "main".to_string(),
            github_token: Secret::value("ghp_test123"),
            repos_dir: std::path::PathBuf::from("/tmp/test-repos"),
        };
        let backend = ZbobrRepoBackendGithub::from_config(config).unwrap();
        assert_eq!(backend.backend_config.repository, "myorg/myrepo");
    }
}
