use async_trait::async_trait;

use crate::task::{Comment, Pipeline, Signal, StackEntry, State, StateOverrideRequest, Task, TaskIdentity};

/// Unicode symbol prepended to every formatted error status message.
pub const ERROR_PREFIX: char = '\u{274C}';

/// Unicode symbol prepended to every formatted question status message.
pub const QUESTION_PREFIX: char = '\u{2753}';

/// Unicode symbol prepended to every formatted pause/confirmation status message.
pub const PAUSE_PREFIX: char = '\u{23F8}';

/// Format a status message with icon, timestamp, and message text.
/// Used for both error and question statuses placed in the STATUS section.
pub fn format_status(
    icon: char,
    ts: &chrono::DateTime<chrono::FixedOffset>,
    message: &str,
) -> String {
    format!(
        "{} {} {}",
        icon,
        crate::context::format_timestamp(ts),
        message
    )
}

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

    /// Poll whether the user has requested a state change via the backend's UI mechanism.
    /// Returns `None` if no change is pending.
    /// Detection logic is backend-specific (e.g., GitHub label changes); the dispatcher
    /// applies the result via `modify_task`, which re-syncs the backend UI state automatically.
    async fn pending_override(&self) -> anyhow::Result<Option<StateOverrideRequest>>;
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
            if !task.state.is_running() && state.is_running() {
                task.status = None;
            }
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

    /// Set the status field directly (pre-formatted string or None to clear).
    async fn set_status(&self, status: Option<String>) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.status = status;
            task
        }))
        .await
    }

    /// Pause the task and set a status message atomically.
    /// It is not possible to set pause without an explanation.
    async fn set_pause_with_status(&self, status: String) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.status = Some(status);
            task.go_pause = true;
            task
        }))
        .await
    }

    /// Pause the task, set a status message, and assign a signal — all atomically.
    /// It is not possible to set pause without an explanation.
    async fn set_pause_with_status_and_signal(
        &self,
        status: String,
        signal: Signal,
    ) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.status = Some(status);
            task.go_pause = true;
            task.signal = Some(signal);
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

    /// Store a report file, deduplicating with `_N` suffix if needed.
    /// Returns the actual filename (without directory prefix).
    async fn store_report(&self, base_name: &str, content: &str) -> anyhow::Result<String>;

    /// Convert a report filename into a browsable URL.
    /// For GitHub: full `https://github.com/…/blob/…` URL.
    /// For local fs: returns the filename as-is.
    fn report_url(&self, filename: &str) -> String;

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

    /// Initialize storage with required labels, etc.
    /// If force is true, overwrites existing labels.
    async fn setup(&self, force: bool) -> anyhow::Result<()>;

    /// Validate connectivity to the task storage.
    async fn validate_connectivity(&self) -> anyhow::Result<()>;

    /// Return a debug string of the backend state.
    fn debug_state(&self) -> String;

    /// Return the "owner/repo" name of the task repository, if available.
    fn task_repo_name(&self) -> Option<String> {
        None
    }

    /// Ensure that labels exist for the given configured pipelines.
    /// This is called during setup to create pipeline labels in the backend.
    /// Only GitHub backend implements this; other backends have a no-op default.
    async fn ensure_pipeline_labels_exist(
        &self,
        _pipelines: &[&Pipeline],
    ) -> anyhow::Result<()> {
        Ok(())
    }
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
    /// Return the full configured repository path (e.g. "owner/repo" or local path).
    fn repository(&self) -> &str;

    /// Return the configured base branch (e.g. "main").
    fn branch(&self) -> &str;

    /// Return the short name of the configured repository (last path component).
    /// Used to compute the workspace subdirectory name.
    fn repo_name(&self) -> &str;

    /// Prepare worktree for the task. Returns Ok(true) if up-to-date,
    /// Ok(false) if merge conflict state.
    async fn update_worktree(
        &self,
        identity: &TaskIdentity,
        workspace_path: &std::path::Path,
        git_user_name: &str,
        git_user_email: &str,
    ) -> anyhow::Result<bool>;

    /// Fetch latest refs from origin with authentication.
    /// Unlike `update_worktree`, this only fetches — no merges, pushes, or PR side effects.
    async fn fetch_refs(&self, identity: &TaskIdentity) -> anyhow::Result<()>;

    /// Ensure a PR exists for the work branch (no push). Returns PR URL.
    async fn ensure_pr_url(
        &self,
        identity: &TaskIdentity,
        body: Option<&str>,
    ) -> anyhow::Result<String>;

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
