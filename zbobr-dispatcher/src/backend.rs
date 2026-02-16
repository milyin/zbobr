use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;

use crate::{Model, Parameter, Stage, Task, Tool, ZbobrError};

// Replace characters that are unsafe or invalid in filenames with '_'.
// Allows ASCII alphanumerics, '-', '_', and '.'.
fn sanitize_filename(name: &str) -> String {
    name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Configure git user settings for a repository.
/// Sets user.name and user.email at the repository level.
pub async fn configure_git_user(
    work_dir: &std::path::Path,
    git_user_name: &str,
    git_user_email: &str,
) -> Result<(), ZbobrError> {
    // Set git user configuration for this repository
    let config_user_status = tokio::process::Command::new("git")
        .args(["config", "--local", "user.name", git_user_name])
        .current_dir(work_dir)
        .status()
        .await
        .map_err(|e| ZbobrError::Other(format!("Failed to set git user.name: {}", e)))?;

    if !config_user_status.success() {
        return Err(ZbobrError::Other("git config user.name failed".to_string()));
    }

    let config_email_status = tokio::process::Command::new("git")
        .args(["config", "--local", "user.email", git_user_email])
        .current_dir(work_dir)
        .status()
        .await
        .map_err(|e| ZbobrError::Other(format!("Failed to set git user.email: {}", e)))?;

    if !config_email_status.success() {
        return Err(ZbobrError::Other("git config user.email failed".to_string()));
    }

    tracing::info!(
        "Configured git user '{}' <{}> in {}",
        git_user_name,
        git_user_email,
        work_dir.display()
    );

    Ok(())
}

/// Create a placeholder file in a branch to ensure it has at least one commit.
/// This is used to prevent GitHub PR API from rejecting branches with no commits.
/// 
/// Creates `.zbobr/{branch_name}` file, stages it, and commits it.
/// Does NOT push — the caller is responsible for pushing if needed.
/// 
/// Git user configuration should be set up before calling this function (via configure_git_user).
/// 
/// # Arguments
/// * `work_dir` - The repository working directory
/// * `branch_name` - The branch name (used for file naming and commit message)
pub async fn create_placeholder_commit(
    work_dir: &std::path::Path,
    branch_name: &str,
) -> Result<(), ZbobrError> {
    let zbobr_dir = work_dir.join(".zbobr");
    let sanitized_branch = sanitize_filename(branch_name);
    let placeholder_path = zbobr_dir.join(&sanitized_branch);

    // Create .zbobr directory
    tokio::fs::create_dir_all(&zbobr_dir)
        .await
        .map_err(|e| ZbobrError::Other(format!("Failed to create .zbobr directory: {}", e)))?;

    // Create placeholder file. On error, emit extended diagnostics to help
    // debug missing directories, permission issues, or odd filesystem states.
    match tokio::fs::File::create(&placeholder_path).await {
        Ok(_) => {}
        Err(e) => {
            let kind = e.kind();
            let raw = e.raw_os_error();

            // Check whether .zbobr exists and whether work_dir looks writable
            let zbobr_exists = tokio::fs::metadata(&zbobr_dir).await.is_ok();
            let work_dir_meta = tokio::fs::metadata(work_dir).await;
            let work_dir_readonly = work_dir_meta
                .as_ref()
                .map(|m| m.permissions().readonly())
                .unwrap_or(false);

            tracing::error!(
                error=%e,
                kind=?kind,
                raw_os_error=?raw,
                placeholder_path=%placeholder_path.display(),
                work_dir=%work_dir.display(),
                zbobr_exists=%zbobr_exists,
                work_dir_readonly=%work_dir_readonly,
                "Failed to create placeholder file with extended diagnostics"
            );

            return Err(ZbobrError::Other(format!(
                "Failed to create placeholder file: {} — attempted path: {} — work_dir: {} — .zbobr exists: {} — work_dir_readonly: {} — kind: {:?} — raw_os_error: {:?}",
                e,
                placeholder_path.display(),
                work_dir.display(),
                zbobr_exists,
                work_dir_readonly,
                kind,
                raw
            )));
        }
    }

    // Stage the file
    let add_status = tokio::process::Command::new("git")
        .args(["add", &format!(".zbobr/{}", sanitized_branch)])
        .current_dir(work_dir)
        .status()
        .await
        .map_err(|e| ZbobrError::Other(format!("Failed to run git add: {}", e)))?;

    if !add_status.success() {
        return Err(ZbobrError::Other("git add for placeholder failed".to_string()));
    }

    // Commit the file
    let commit_msg = format!("chore: add branch placeholder {}", branch_name);
    let commit_status = tokio::process::Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(work_dir)
        .status()
        .await
        .map_err(|e| ZbobrError::Other(format!("Failed to run git commit: {}", e)))?;

    if !commit_status.success() {
        return Err(ZbobrError::Other("git commit for placeholder failed".to_string()));
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait Backend: Send + Sync {
    /// Get a task by ID.
    async fn get_task(&self, id: u64) -> Result<Task, ZbobrError>;

    /// Create a new task. Returns the task ID.
    #[allow(clippy::too_many_arguments)]
    async fn create_task(
        &self,
        title: &str,
        description: &str,
        stage: Stage,
        tool: Option<Tool>,
        model: Option<Model>,
        parameters: HashMap<Parameter, String>,
    ) -> Result<u64, ZbobrError>;

    /// Close a task.
    async fn close_task(&self, id: u64) -> Result<(), ZbobrError>;

    /// Get all comments on a task as formatted discussion.
    async fn get_task_comments(&self, id: u64) -> Result<Vec<String>, ZbobrError>;

    /// Post a comment on a task with role and hostname metadata.
    async fn post_task_comment(
        &self,
        id: u64,
        body: &str,
        role: &str,
        hostname: &str,
    ) -> Result<(), ZbobrError>;

    /// Read-modify-write the task atomically.
    ///
    /// Takes `Task` by value and returns the modified version to avoid
    /// reference lifetime issues with `async_trait`.
    async fn modify_task(
        &self,
        id: u64,
        mutate: Box<dyn FnOnce(Task) -> Task + Send>,
    ) -> Result<(), ZbobrError>;

    /// List open tasks with a given stage, optionally filtered by tool.
    async fn list_tasks_by_stage(
        &self,
        stage: Stage,
        tool: Option<Tool>,
    ) -> Result<Vec<Task>, ZbobrError>;

    /// Check if a task is closed.
    async fn is_task_closed(&self, id: u64) -> Result<bool, ZbobrError>;

    /// Check if a file exists in the task repo.
    async fn repo_file_exists(&self, path: &str) -> Result<bool, ZbobrError>;

    /// Create or update a file in the task repo.
    async fn create_repo_file(
        &self,
        path: &str,
        content: &str,
        commit_message: &str,
    ) -> Result<(), ZbobrError>;

    /// Ensure the task repo exists.
    async fn ensure_task_repo_exists(&self) -> Result<(), ZbobrError>;

    /// Clone a repo into the workspace, checkout specific branch, set up fork remote.
    /// Returns the local path.
    async fn clone_and_setup(
        &self,
        target_repo: &str,
        branch: &str,
        task_id: u64,
    ) -> Result<PathBuf, ZbobrError>;

    /// Clone a repo and checkout specific branch for read-only investigation (no fork).
    async fn clone_readonly(
        &self,
        target_repo: &str,
        branch: &str,
        task_id: u64,
    ) -> Result<PathBuf, ZbobrError>;

    /// Parse PR reference (URL or owner/repo#123) to (repo, branch).
    async fn parse_pr_to_repo_branch(&self, pr_ref: &str) -> Result<(String, String), ZbobrError>;

    /// Push the current branch to the fork remote and create a PR.
    async fn push_and_create_pr(
        &self,
        target_repo: &str,
        task_id: u64,
    ) -> Result<String, ZbobrError>;

    /// Ensure the fork is synchronized with the upstream `target_repo` on `branch`.
    /// This performs a server-side sync (same as GitHub "Sync fork" button) and is
    /// not a local operation on the cloned copy.
    async fn sync_fork(&self, target_repo: &str, branch: &str) -> Result<(), ZbobrError>;

    /// Create a PR from work_branch to destination_branch in the fork repo.
    /// `repo_name` is just the repository name (e.g. "myrepo"), not a full "owner/repo" path.
    /// The backend determines the fork owner internally.
    /// Returns the PR URL on success, or empty string on stub backend.
    async fn create_pr_in_fork(
        &self,
        repo_name: &str,
        work_branch: &str,
        destination_branch: &str,
        task_id: u64,
    ) -> Result<String, ZbobrError>;

    /// Replace origin remote with the fork URL and push a work branch.
    /// The backend determines the fork owner and constructs the fork URL internally.
    async fn setup_fork_remote_and_push(
        &self,
        work_dir: &std::path::Path,
        target_repo: &str,
        work_branch: &str,
    ) -> Result<(), ZbobrError>;

    // -- Setup methods --

    /// List all stages (milestones) in the task repo.
    async fn list_stages(&self) -> Result<Vec<(u64, String)>, ZbobrError>;

    /// Create a stage (milestone) in the task repo.
    async fn create_stage(&self, title: &str, description: &str) -> Result<(), ZbobrError>;

    /// Delete a stage by its number.
    async fn delete_stage(&self, number: u64) -> Result<(), ZbobrError>;

    /// List all labels in the task repo.
    async fn list_labels(&self) -> Result<Vec<String>, ZbobrError>;

    /// Create a label in the task repo.
    async fn create_label(
        &self,
        name: &str,
        color: &str,
        description: &str,
    ) -> Result<(), ZbobrError>;

    /// Initialize the task repository with stages and labels.
    /// If force is true, overwrites existing labels.
    async fn setup_repository(&self, force: bool) -> Result<(), ZbobrError>;

    /// Validate that the backend can reach required resources (fork owner, task repo, etc.).
    async fn validate_connectivity(&self) -> Result<(), ZbobrError>;

    /// Return a debug string of the backend state.
    fn debug_state(&self) -> String;
}
