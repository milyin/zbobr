use std::collections::HashMap;

use async_trait::async_trait;

use crate::task::{
    ChecklistItem, Comment, CommentType, Model, Parameter, Role, Signal, Stage, Task, TaskIdentity,
};
use crate::Tool;

/// Read-only handle to a task. Returned by `TaskBackend::get_task()` and `TaskBackend::list_tasks()`.
#[async_trait]
pub trait TaskWeak: Send + Sync {
    fn task_id(&self) -> u64;

    /// Get a snapshot of the full task state.
    async fn snapshot(&self) -> anyhow::Result<Task>;

    /// Try to acquire exclusive write access.
    /// Fails if another TaskMut is held for this task.
    async fn upgrade(&self) -> anyhow::Result<Box<dyn TaskMut>>;

    /// Read comments (read-only).
    async fn get_comments(&self) -> anyhow::Result<Vec<Comment>>;
}

/// Exclusive mutable handle. Obtained via `TaskWeak::upgrade()`. Dropping releases the lock.
#[async_trait]
pub trait TaskMut: Send + Sync {
    fn task_id(&self) -> u64;
    async fn snapshot(&self) -> anyhow::Result<Task>;

    /// Core mutation primitive — reads task, applies closure, writes back.
    async fn modify_task(
        &self,
        mutate: Box<dyn FnOnce(Task) -> Task + Send>,
    ) -> anyhow::Result<()>;

    /// Close the task.
    async fn close(&self) -> anyhow::Result<()>;

    // --- Default setter methods (use modify_task) ---

    async fn set_stage(&self, stage: Stage) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.stage = stage;
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

    async fn set_conflict(&self, conflict: bool) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.conflict = conflict;
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

    async fn set_parameter(&self, param: Parameter, value: Option<String>) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            if let Some(v) = value {
                task.parameters.insert(param, v);
            } else {
                task.parameters.remove(&param);
            }
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

    async fn set_checklist(&self, items: Vec<ChecklistItem>) -> anyhow::Result<()> {
        self.modify_task(Box::new(move |mut task| {
            task.checklist = items;
            task
        }))
        .await
    }

    /// Post a structured comment (requires exclusive access).
    async fn post_comment(
        &self,
        comment_type: CommentType,
        role: Option<Role>,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
        body: &str,
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

    /// List all open tasks.
    async fn list_tasks(&self) -> anyhow::Result<Vec<Box<dyn TaskWeak>>> {
        // Default: list all stages except Done
        let mut all = Vec::new();
        for stage in &[
            Stage::Pending,
            Stage::Preparing,
            Stage::Planning,
            Stage::Working,
            Stage::Reviewing,
            Stage::Testing,
            Stage::Merging,
        ] {
            let mut tasks = self.list_tasks_by_stage(*stage).await?;
            all.append(&mut tasks);
        }
        Ok(all)
    }

    /// List open tasks with a given stage.
    async fn list_tasks_by_stage(&self, stage: Stage) -> anyhow::Result<Vec<Box<dyn TaskWeak>>>;

    /// Create a new task. Returns the task ID.
    async fn create_task(
        &self,
        title: &str,
        description: &str,
        stage: Stage,
        parameters: HashMap<Parameter, String>,
    ) -> anyhow::Result<u64>;

    /// Initialize storage with required stages, labels, etc.
    /// If force is true, overwrites existing labels.
    async fn setup(&self, force: bool) -> anyhow::Result<()>;

    /// Validate connectivity to the task storage.
    async fn validate_connectivity(&self) -> anyhow::Result<()>;

    /// Return a debug string of the backend state.
    fn debug_state(&self) -> String;
}

/// Extension trait for higher-level operations built on TaskBackend + TaskWeak/TaskMut.
#[async_trait]
pub trait TaskBackendExt: TaskBackend {
    /// Set the stage of a task.
    async fn set_task_stage(&self, id: u64, stage: Stage) -> anyhow::Result<()> {
        let weak = self.get_task(id).await?;
        let mutable = weak.upgrade().await?;
        mutable.set_stage(stage).await
    }

    /// Set the signal on a task.
    async fn set_task_signal(&self, id: u64, signal: Option<Signal>) -> anyhow::Result<()> {
        let weak = self.get_task(id).await?;
        let mutable = weak.upgrade().await?;
        mutable.set_signal(signal).await
    }
}

impl<T: TaskBackend + ?Sized> TaskBackendExt for T {}

#[async_trait]
pub trait WorktreeBackend: Send + Sync {
    /// Prepare worktree for the task. Returns Ok(true) if up-to-date,
    /// Ok(false) if merge conflict state.
    async fn update_worktree(
        &self,
        identity: &TaskIdentity,
        workspace_path: &std::path::Path,
    ) -> anyhow::Result<bool>;

    /// Push work branch and ensure PR exists. Returns PR URL.
    async fn update_pr(&self, identity: &TaskIdentity) -> anyhow::Result<String>;

    /// Validate connectivity to the repo hosting service.
    async fn validate_connectivity(&self) -> anyhow::Result<()>;

    /// Return a debug string of the backend state.
    fn debug_state(&self) -> String;
}
