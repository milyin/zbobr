use async_trait::async_trait;

use crate::task::{
    Comment, Model, Signal, StackEntry, State, Task, TaskIdentity, Tool,
};

/// Read-only handle to a task. Returned by `TaskBackend::get_task()` and `TaskBackend::list_tasks()`.
#[async_trait]
pub trait TaskWeak: Send + Sync {
    fn task_id(&self) -> u64;

    /// Get a snapshot of the full task state.
    ///
    /// When `refresh` is `false`, implementations may return a cached snapshot.
    /// When `refresh` is `true`, implementations must reload from the backing store
    /// and refresh any saved snapshot cache.
    async fn snapshot(&self, refresh: bool) -> anyhow::Result<Task>;

    /// Try to acquire exclusive write access.
    /// Fails if another TaskMut is held for this task.
    async fn upgrade(&self) -> anyhow::Result<Box<dyn TaskMut>>;

    /// Read comments (read-only).
    async fn get_comments(&self) -> anyhow::Result<Vec<Comment>>;

    /// Read a stored report file by name.
    async fn read_report(&self, name: &str) -> anyhow::Result<String>;
}

/// Exclusive mutable handle. Obtained via `TaskWeak::upgrade()`. Dropping releases the lock.
#[async_trait]
pub trait TaskMut: Send + Sync {
    fn task_id(&self) -> u64;
    async fn snapshot(&self, refresh: bool) -> anyhow::Result<Task>;

    /// Core mutation primitive — reads task, applies closure, writes back.
    async fn modify_task(&self, mutate: Box<dyn FnOnce(Task) -> Task + Send>)
    -> anyhow::Result<()>;

    /// Close the task.
    async fn close(&self) -> anyhow::Result<()>;

    // --- Default setter methods (use modify_task) ---

    async fn set_state(&self, state: State) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.state = state;
            task
        }))
        .await
    }

    async fn set_signal(&self, signal: Option<Signal>) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.signal = signal;
            task
        }))
        .await
    }

    async fn set_stack(&self, stack: Vec<StackEntry>) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.stack = stack;
            task
        }))
        .await
    }

    async fn set_pause(&self, pause: bool) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.pause = pause;
            task
        }))
        .await
    }

    async fn set_confirm(&self, confirm: bool) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.confirm = confirm;
            task
        }))
        .await
    }

    async fn set_error(&self, error: Option<String>) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.error = error;
            task
        }))
        .await
    }

    async fn set_destination_repository(&self, repo: Option<String>) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.destination_repository = repo;
            task
        }))
        .await
    }

    async fn set_destination_branch(&self, branch: Option<String>) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.destination_branch = branch;
            task
        }))
        .await
    }

    async fn set_work_branch(&self, branch: Option<String>) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.work_branch = branch;
            task
        }))
        .await
    }

    async fn set_description(&self, desc: String) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.description = desc;
            task
        }))
        .await
    }

    /// Post a structured comment (requires exclusive access).
    /// If `report_text` is provided, stores it as a report file and sets `report_name`
    /// on the comment. The filename is generated from the comment's pipeline, run id,
    /// stage, and outcome (derived from the body's `[tool_name]` tag).
    async fn post_comment(
        &self,
        stage: &str,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
        body: &str,
        pipeline: &str,
        pipeline_run_id: u64,
        caller_pipeline: Option<&str>,
        caller_pipeline_run_id: Option<u64>,
        report_text: Option<&str>,
        prompt_text: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Release exclusive access, return read-only handle.
    fn downgrade(self: Box<Self>) -> Box<dyn TaskWeak>;
}

/// TaskBackend: stores and manages tasks, their metadata, comments, and lifecycle.
///
/// Implementations:
/// - GitHub: Tasks as Issues, stages as Milestones, signals/tools/models as Labels
/// - Directory: Tasks as YAML files, stages as subdirectories
#[async_trait]
pub trait TaskBackend: Send + Sync {
    /// Get a read-only handle to a task by ID.
    async fn get_task(&self, id: u64) -> anyhow::Result<Box<dyn TaskWeak>>;

    /// List all open tasks (state != "DONE").
    async fn list_tasks(&self) -> anyhow::Result<Vec<Box<dyn TaskWeak>>>;

    /// Create a new task. Returns the task ID.
    async fn create_task(
        &self,
        title: &str,
        description: &str,
        state: State,
    ) -> anyhow::Result<u64>;

    /// Initialize storage with required stages, labels, etc.
    /// If force is true, overwrites existing labels.
    /// `signal_labels` lists the signal label names that should exist (e.g. "signal:go_working").
    async fn setup(&self, force: bool, signal_labels: &[String]) -> anyhow::Result<()>;

    /// Validate connectivity to the task storage.
    async fn validate_connectivity(&self) -> anyhow::Result<()>;

    /// Return a debug string of the backend state.
    fn debug_state(&self) -> String;
}

/// Extension trait for higher-level operations built on TaskBackend + TaskWeak/TaskMut.
#[async_trait]
pub trait TaskBackendExt: TaskBackend {
    /// Set the state of a task.
    async fn set_task_state(&self, id: u64, state: State) -> anyhow::Result<()> {
        let weak = self.get_task(id).await?;
        let mutable = weak.upgrade().await?;
        mutable.set_state(state).await
    }

    /// Set the signal on a task.
    async fn set_task_signal(&self, id: u64, signal: Option<Signal>) -> anyhow::Result<()> {
        let weak = self.get_task(id).await?;
        let mutable = weak.upgrade().await?;
        mutable.set_signal(signal).await
    }

    /// Set the stack on a task.
    async fn set_task_stack(&self, id: u64, stack: Vec<StackEntry>) -> anyhow::Result<()> {
        let weak = self.get_task(id).await?;
        let mutable = weak.upgrade().await?;
        mutable.set_stack(stack).await
    }
}

impl<T: TaskBackend + ?Sized> TaskBackendExt for T {}

impl<T: TaskBackend + 'static> From<T> for Box<dyn TaskBackend> {
    fn from(backend: T) -> Self {
        Box::new(backend)
    }
}

#[async_trait]
pub trait WorktreeBackend: Send + Sync {
    /// Prepare worktree for the task. Returns Ok(true) if up-to-date,
    /// Ok(false) if merge conflict state.
    async fn update_worktree(
        &self,
        identity: &TaskIdentity,
        workspace_path: &std::path::Path,
        git_user_name: &str,
        git_user_email: &str,
    ) -> anyhow::Result<bool>;

    /// Ensure a PR exists for the work branch (no push). Returns PR URL.
    async fn ensure_pr_url(&self, identity: &TaskIdentity) -> anyhow::Result<String>;

    /// Validate connectivity to the repo hosting service.
    async fn validate_connectivity(&self) -> anyhow::Result<()>;

    /// Return a debug string of the backend state.
    fn debug_state(&self) -> String;
}

impl<T: WorktreeBackend + 'static> From<T> for Box<dyn WorktreeBackend> {
    fn from(backend: T) -> Self {
        Box::new(backend)
    }
}
