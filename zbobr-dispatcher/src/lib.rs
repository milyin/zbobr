pub mod backend;
pub mod cleanup;
pub mod cli;
pub mod config;
pub mod generic_config;
pub mod mcp;
pub mod prompts;
pub mod setup;
pub mod task;
pub mod task_dir;
pub mod tool_executor;

use std::{collections::HashMap, sync::Arc};

pub use cli::{
    Command, ConfigFileArg, GlobalArgs, TaskSubcommand, parse_cli, print_task,
    process_task_by_stage, run_command, run_manager_loop, run_zbobr,
};
pub use config::{
    ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml, ZbobrExecutorArgs,
    ZbobrExecutorConfig, ZbobrExecutorToml, ZbobrRepoBackendConfig, ZbobrTaskBackendConfig,
};
pub use generic_config::{GenericConfig, GenericConfigArgs, GenericConfigToml};
pub use mcp::{
    MergerMcp, PlannerMcp, PreparatorMcp, ReviewerMcp, TesterMcp, WorkerMcp, merger_instructions,
    planner_instructions, preparator_instructions, reviewer_instructions, tester_instructions,
    worker_instructions,
};
pub use prompts::{Prompts, build_full_prompt, load_prompts, resolve_prompts};
pub use task::{
    ChecklistItem, Comment, CommentType, Model, Parameter, RoleSession, Signal, Stage, Task,
    TaskSession, Tool,
};
pub use task_dir::TaskDir;
pub use tool_executor::ToolExecutor;
pub use zbobr_api::config::{BackendConfig, Config};

use crate::backend::{TaskBackend, WorktreeBackend};

/// Central struct holding configuration and backend.
pub struct ZbobrDispatcher<T: TaskBackend + ?Sized, R: WorktreeBackend + ?Sized> {
    config: Arc<ZbobrDispatcherConfig>,
    pub(crate) task_backend: Arc<T>,
    pub(crate) repo_backend: Arc<R>,
    /// Per-task mutexes to serialize concurrent read-modify-write cycles
    /// for the same task within this process.
    task_locks: Arc<std::sync::Mutex<HashMap<u64, Arc<tokio::sync::Mutex<()>>>>>,
}

impl<T: TaskBackend + ?Sized, R: WorktreeBackend + ?Sized> Clone for ZbobrDispatcher<T, R> {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            task_backend: Arc::clone(&self.task_backend),
            repo_backend: Arc::clone(&self.repo_backend),
            task_locks: Arc::clone(&self.task_locks),
        }
    }
}

/// Convenience type alias for using the dispatcher with dynamic dispatch.
pub type ZbobrDispatcherDyn = ZbobrDispatcher<dyn TaskBackend, dyn WorktreeBackend>;

impl<T: TaskBackend + ?Sized, R: WorktreeBackend + ?Sized> ZbobrDispatcher<T, R> {
    /// Create a new Zbobr instance from config and pre-built backends.
    /// Used primarily in tests.
    pub fn new_with_backends(
        config: ZbobrDispatcherConfig,
        task_backend: Arc<T>,
        repo_backend: Arc<R>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            task_backend,
            repo_backend,
            task_locks: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    pub fn config(&self) -> &ZbobrDispatcherConfig {
        &self.config
    }

    /// Get or create a per-task async mutex for serializing read-modify-write cycles.
    pub(crate) fn task_lock(&self, task_id: u64) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.task_locks.lock().unwrap();
        locks
            .entry(task_id)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// Validate that both backends can reach required resources.
    pub async fn validate_connectivity(&self) -> anyhow::Result<()> {
        self.task_backend.validate_connectivity().await?;
        self.repo_backend.validate_connectivity().await?;
        Ok(())
    }

    // -- Delegate to TaskBackend --

    pub async fn get_task(&self, id: u64) -> anyhow::Result<Task> {
        self.task_backend.get_task(id).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_task(
        &self,
        title: &str,
        description: &str,
        stage: Stage,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
    ) -> anyhow::Result<u64> {
        self.create_task_with_confirm(
            title,
            description,
            stage,
            destination_repository,
            destination_branch,
            false,
        )
        .await
    }

    /// Like `create_task` but also set the confirm flag on the new task when
    /// `confirm == true`.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_task_with_confirm(
        &self,
        title: &str,
        description: &str,
        stage: Stage,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
        confirm: bool,
    ) -> anyhow::Result<u64> {
        let id = {
            let mut parameters = std::collections::HashMap::new();
            if let Some(repo) = destination_repository {
                parameters.insert(Parameter::DestinationRepository, repo);
            }
            if let Some(branch) = destination_branch {
                parameters.insert(Parameter::DestinationBranch, branch);
            }
            self.task_backend
                .create_task(title, description, stage, parameters)
                .await?
        };
        if confirm {
            self.modify_task(
                id,
                Box::new(|mut task| {
                    task.confirm = true;
                    task
                }),
            )
            .await?;
        }
        Ok(id)
    }

    pub async fn close_task(&self, id: u64) -> anyhow::Result<()> {
        self.task_backend.close_task(id).await
    }

    pub async fn get_task_comments(&self, id: u64) -> anyhow::Result<Vec<Comment>> {
        self.task_backend.get_task_comments(id).await
    }

    /// Fetch comments and description for a task, then extract the history chunk
    /// at the given `offset` using [`zbobr_api::extract_history_chunk`].
    pub async fn get_history(
        &self,
        id: u64,
        offset: Option<usize>,
    ) -> anyhow::Result<zbobr_api::HistoryChunk> {
        let comments = self.get_task_comments(id).await?;
        let desc = self
            .get_task(id)
            .await
            .map(|t| t.description)
            .unwrap_or_default();
        zbobr_api::extract_history_chunk(comments, &desc, offset)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn post_task_comment(
        &self,
        id: u64,
        comment_type: CommentType,
        role: Option<zbobr_api::Role>,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
        body: &str,
    ) -> anyhow::Result<()> {
        self.task_backend
            .post_task_comment(id, comment_type, role, hostname, tool, model, body)
            .await
    }

    pub async fn set_task_stage(&self, id: u64, stage: Stage) -> anyhow::Result<()> {
        self.task_backend
            .modify_task(
                id,
                Box::new(move |mut task| {
                    task.stage = stage;
                    task
                }),
            )
            .await
    }

    pub async fn set_task_signal(&self, id: u64, signal: Option<Signal>) -> anyhow::Result<()> {
        self.task_backend
            .modify_task(
                id,
                Box::new(move |mut task| {
                    task.signal = signal;
                    task
                }),
            )
            .await
    }

    pub async fn list_tasks_by_stage(&self, stage: Stage) -> anyhow::Result<Vec<Task>> {
        self.task_backend.list_tasks_by_stage(stage).await
    }

    pub async fn is_task_closed(&self, id: u64) -> anyhow::Result<bool> {
        self.task_backend.is_task_closed(id).await
    }

    pub async fn modify_task(
        &self,
        id: u64,
        mutate: Box<dyn FnOnce(Task) -> Task + Send>,
    ) -> anyhow::Result<()> {
        self.task_backend.modify_task(id, mutate).await
    }

    pub async fn setup_repository(&self, force: bool) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.config.workspaces)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to create workspaces directory '{}': {}",
                    self.config.workspaces.display(),
                    e
                )
            })?;
        tracing::info!(
            "Workspaces directory ready: {}",
            self.config.workspaces.display()
        );
        self.task_backend.setup(force).await
    }

    // -- Delegate to WorktreeBackend --

    /// Extract repo name from a remote repo path (last path component).
    fn extract_repo_name(remote_repo: &str) -> &str {
        remote_repo.rsplit('/').next().unwrap_or(remote_repo)
    }

    /// Prepare a worktree for the given task. Constructs workspace_path as
    /// `TaskDir::new(workspaces, task_id)/repo_name` and delegates to the backend.
    pub async fn update_worktree(
        &self,
        remote_repo: &str,
        base_branch: &str,
        work_branch: &str,
        task_id: u64,
    ) -> anyhow::Result<bool> {
        let repo_name = Self::extract_repo_name(remote_repo);
        let task_dir = TaskDir::new(&self.config.workspaces, task_id);
        let workspace_path = task_dir.path().join(repo_name);
        self.repo_backend
            .update_worktree(remote_repo, base_branch, work_branch, &workspace_path)
            .await
    }

    /// Sync remote state and return PR URL for the given work branch.
    pub async fn update_pr(&self, work_branch: &str) -> anyhow::Result<String> {
        self.repo_backend.update_pr(work_branch).await
    }

    // -- Combined state --

    pub fn debug_state(&self) -> String {
        format!(
            "task_backend: {}, repo_backend: {}",
            self.task_backend.debug_state(),
            self.repo_backend.debug_state()
        )
    }
}

// Implementation specific to the dynamic dispatcher type.
impl ZbobrDispatcherDyn {
    /// Create a TaskSession bound to a specific task (full dispatcher access).
    pub fn task_session(&self, task_id: u64) -> TaskSession {
        TaskSession::new(self.clone(), task_id)
    }

    /// Create a RoleSession bound to a specific task (restricted MCP tool access).
    pub fn role_session(&self, task_id: u64) -> RoleSession {
        RoleSession::new(self.clone(), task_id)
    }
}
