use crate::Tool;
use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;

use crate::task::{Comment, CommentType, Model, Parameter, Role, Signal, Stage, Task};

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

    /// List open tasks with a given stage.
    async fn list_tasks_by_stage(&self, stage: Stage) -> anyhow::Result<Vec<Task>>;

    // -- Discussion --

    /// Get all comments on a task as structured Comment objects.
    async fn get_task_comments(&self, id: u64) -> anyhow::Result<Vec<Comment>>;

    /// Post a comment on a task with structured metadata.
    async fn post_task_comment(
        &self,
        id: u64,
        comment_type: CommentType,
        role: Option<Role>,
        hostname: &str,
        tool: Option<Tool>,
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

#[async_trait]
pub trait WorktreeBackend: Send + Sync {
    /// **Precondition**: `work_branch` must differ from `base_branch`. Passing the same
    /// value for both is a caller error and implementations should return `Err`.
    ///
    /// 1. Clones remote repository as `--bare` to an own location if not already cloned.
    ///    It's important to notice that this clone is not necessarity the direct clone
    ///    of the `remote_repo` in the parameter. It's up to backend how to proxy the
    ///    remote repository, e.g. by creating a fork in own account and locally cloning it.
    ///
    /// 2. Creates in the `workspace_path` a new worktree linked to the main clone for
    /// the branch `work_branch` with `base_branch` as the upstream.
    ///
    /// Conditions to be met for successful worktree creation:
    ///
    /// The `base_branch` must exist in the main worktree.
    /// The `work_branch`
    ///  - should either not exist or be an ancestor of `base_branch` in the main worktree.
    ///  - if the branch is already checked out in a **functional** worktree (directory exists
    ///    and contains a `.git` entry) at `workspace_path`, the worktree is considered ready
    ///    and no action is needed.
    ///  - if the branch is checked out in a functional worktree at a **different** path,
    ///    the implementation must return `Err` (concurrent use by another workspace).
    ///  - if the branch is registered in a **non-functional** worktree (empty or missing
    ///    directory, no `.git` entry), the stale reference must be removed before creating
    ///    a new worktree at `workspace_path`.
    ///  - the `workspace_path` should either not exist or contain the worktree with the
    ///    `work_branch` selected
    ///
    /// Corresponding delete operation is not required: the worktree can be removed with
    /// `git worktree remove` command
    ///
    /// Returns boolean value indicating whether the worktree branch is
    /// successfully created and merged to recent remote updates
    ///
    /// I.e return values:
    /// - `Ok(true)` means the worktree is ready to use and up to date with the `base_branch`.
    /// - `Ok(false)` means the worktree is in a merging conflict state
    /// - `Err` means the worktree is not ready to use, e.g. due to validation
    ///    failure or creation failure.
    ///
    /// The merging `base_branch` into `work_branch` is the responsibility of the caller.
    async fn update_worktree(
        &self,
        remote_repo: &str,
        base_branch: &str,
        work_branch: &str,
        workspace_path: &std::path::Path,
    ) -> anyhow::Result<bool>;

    /// Push the work branch to its remote and ensure a PR exists.
    ///
    /// Returns the up-to-date URL of the PR for the given work branch.
    /// It's up to backend what is considered as the PR URL, this information is only
    /// for representing the current status of the work for the user.
    /// For the GitHub backend, this is the URL of the PR in the GitHub UI.
    /// For the filesystem backend, this can be just the path to the worktree directory.
    ///
    /// If no PR exists yet, the backend should create one targeting `base_branch`
    /// in `destination_repo`.
    async fn update_pr(
        &self,
        work_branch: &str,
        destination_repo: &str,
        base_branch: &str,
    ) -> anyhow::Result<String>;

    /// Validate connectivity to the repo hosting service.
    async fn validate_connectivity(&self) -> anyhow::Result<()>;

    /// Return a debug string of the backend state.
    fn debug_state(&self) -> String;
}

/// Extension trait for higher-level operations built on [`TaskBackend::modify_task`].
#[async_trait]
pub trait TaskBackendExt: TaskBackend {
    /// Set the stage of a task.
    async fn set_task_stage(&self, id: u64, stage: Stage) -> anyhow::Result<()> {
        self.modify_task(
            id,
            Box::new(move |mut task| {
                task.stage = stage;
                task
            }),
        )
        .await
    }

    /// Set the signal on a task.
    async fn set_task_signal(&self, id: u64, signal: Option<Signal>) -> anyhow::Result<()> {
        self.modify_task(
            id,
            Box::new(move |mut task| {
                task.signal = signal;
                task
            }),
        )
        .await
    }
}

impl<T: TaskBackend + ?Sized> TaskBackendExt for T {}
