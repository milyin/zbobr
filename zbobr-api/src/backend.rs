use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;

use crate::task::{Comment, CommentType, Model, Parameter, Role, Stage, Task, Tool};

/// TaskBackend: stores and manages tasks, their metadata, comments, and lifecycle.
///
/// Implementations:
/// - GitHub: Tasks as Issues, stages as Milestones, signals/tools/models as Labels
/// - Directory: Tasks as JSON files, stages as subdirectories
#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait TaskBackend: Send + Sync {
    // -- Core CRUD --

    /// Get a task by ID.
    async fn get_task(&self, id: u64) -> anyhow::Result<Task>;

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
    ) -> anyhow::Result<u64>;

    /// Close a task.
    async fn close_task(&self, id: u64) -> anyhow::Result<()>;

    /// Check if a task is closed.
    async fn is_task_closed(&self, id: u64) -> anyhow::Result<bool>;

    // -- Atomic modification --

    /// Read-modify-write the task atomically.
    ///
    /// Takes `Task` by value and returns the modified version to avoid
    /// reference lifetime issues with `async_trait`.
    async fn modify_task(
        &self,
        id: u64,
        mutate: Box<dyn FnOnce(Task) -> Task + Send>,
    ) -> anyhow::Result<()>;

    // -- Queries --

    /// List open tasks with a given stage, optionally filtered by tool.
    async fn list_tasks_by_stage(
        &self,
        stage: Stage,
        tool: Option<Tool>,
    ) -> anyhow::Result<Vec<Task>>;

    // -- Discussion --

    /// Get all comments on a task as structured Comment objects.
    async fn get_task_comments_structured(&self, id: u64) -> anyhow::Result<Vec<Comment>>;

    /// Post a comment on a task with structured metadata.
    async fn post_task_comment_structured(
        &self,
        id: u64,
        comment_type: CommentType,
        role: Option<Role>,
        hostname: &str,
        model: Option<Model>,
        body: &str,
    ) -> anyhow::Result<()>;

    // -- Lifecycle --

    /// Initialize storage with required stages, labels, etc.
    /// If force is true, overwrites existing labels.
    async fn setup(&self, force: bool) -> anyhow::Result<()>;

    /// Validate connectivity to the task storage.
    async fn validate_connectivity(&self) -> anyhow::Result<()>;

    /// Return a debug string of the backend state.
    fn debug_state(&self) -> String;
}

/// RepoBackend: manages code repositories — cloning, forking, branching, and PRs.
///
/// Implementations:
/// - GitHub: Forks via GitHub API, clones via `gh`, PRs via Pulls API
/// - Directory: Local git repos, direct push, no fork/PR concept
#[async_trait]
pub trait RepoBackend: Send + Sync {
    // -- Clone/checkout --

    /// Clone a repo into the workspace, checkout work branch, sync destination branch, set up fork remote.
    /// Ensures the fork is synced with destination_branch, the local destination_branch is force-reset
    /// to match origin, and the work_branch is checked out ready for merging.
    /// Returns the local path.
    async fn clone_and_setup(
        &self,
        target_repo: &str,
        work_branch: &str,
        destination_branch: &str,
        workspace_path: &std::path::Path,
    ) -> anyhow::Result<PathBuf>;

    /// Clone a repo and checkout specific branch for read-only investigation (no fork).
    async fn clone_readonly(
        &self,
        target_repo: &str,
        branch: &str,
        workspace_path: &std::path::Path,
    ) -> anyhow::Result<PathBuf>;

    // -- Fork management --

    /// Replace origin remote with the fork URL and push a work branch.
    /// The backend determines the fork owner and constructs the fork URL internally.
    async fn setup_fork_remote_and_push(
        &self,
        work_dir: &std::path::Path,
        target_repo: &str,
        work_branch: &str,
    ) -> anyhow::Result<()>;

    // -- PR operations --

    /// Create (or find existing) branch reference on the remote and return a PR URL.
    ///
    /// For FS backend: returns the work directory path string (no git push).
    /// For GitHub backend: pushes the branch (adding a placeholder commit if the branch has no
    /// new commits relative to `destination_branch`), creates a draft PR, or finds the existing
    /// PR URL if GitHub returns 422, and returns the PR `html_url`.
    async fn ensure_branch_and_pr(
        &self,
        target_repo: &str,
        workspace_path: &std::path::Path,
        work_branch: &str,
        destination_branch: &str,
        pr_title: &str,
    ) -> anyhow::Result<String>;

    /// Push current work branch commits to the remote (fork or origin).
    ///
    /// FS backend: no-op (`Ok(())`).
    /// GitHub backend: git push to fork or origin (same remote detection as `ensure_branch_and_pr`).
    async fn push_branch(
        &self,
        target_repo: &str,
        workspace_path: &std::path::Path,
        work_branch: &str,
    ) -> anyhow::Result<()>;

    /// Create a PR from work_branch to destination_branch in the fork repo.
    /// `repo_name` is just the repository name (e.g. "myrepo"), not a full "owner/repo" path.
    /// The backend determines the fork owner internally.
    /// `pr_title` and `pr_body` are provided by the caller (decoupled from task storage).
    /// Returns the PR URL.
    async fn create_pr_in_fork(
        &self,
        repo_name: &str,
        work_branch: &str,
        destination_branch: &str,
        pr_title: &str,
        pr_body: &str,
    ) -> anyhow::Result<String>;

    /// Parse PR reference (URL or owner/repo#123) to (repo, branch).
    async fn parse_pr_to_repo_branch(&self, pr_ref: &str) -> anyhow::Result<(String, String)>;

    // -- Lifecycle --

    /// Validate connectivity to the repo hosting service (fork owner accessible, etc.).
    async fn validate_connectivity(&self) -> anyhow::Result<()>;

    /// Return a debug string of the backend state.
    fn debug_state(&self) -> String;
}
