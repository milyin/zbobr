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

use std::sync::Arc;

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

/// Holds the task and worktree backends as trait objects.
#[derive(Clone)]
pub struct Backends {
    task_backend: Arc<dyn TaskBackend>,
    repo_backend: Arc<dyn WorktreeBackend>,
}

impl Backends {
    pub fn new(
        task_backend: Arc<dyn TaskBackend>,
        repo_backend: Arc<dyn WorktreeBackend>,
    ) -> Self {
        Self {
            task_backend,
            repo_backend,
        }
    }

    /// Direct access to the task backend.
    pub fn tasks(&self) -> &dyn TaskBackend {
        self.task_backend.as_ref()
    }

    /// Direct access to the worktree backend.
    pub fn worktree(&self) -> &dyn WorktreeBackend {
        self.repo_backend.as_ref()
    }

    /// Validate that both backends can reach required resources.
    pub async fn validate_connectivity(&self) -> anyhow::Result<()> {
        self.tasks().validate_connectivity().await?;
        self.worktree().validate_connectivity().await?;
        Ok(())
    }

    pub fn debug_state(&self) -> String {
        format!(
            "task_backend: {}, repo_backend: {}",
            self.tasks().debug_state(),
            self.worktree().debug_state()
        )
    }

    /// Fetch comments and description for a task, then extract the history chunk
    /// at the given `offset` using [`zbobr_api::extract_history_chunk`].
    pub async fn get_history(
        &self,
        id: u64,
        offset: Option<usize>,
    ) -> anyhow::Result<zbobr_api::HistoryChunk> {
        let weak = self.tasks().get_task(id).await?;
        let comments = weak.get_comments().await?;
        let task = weak.snapshot().await?;
        zbobr_api::extract_history_chunk(comments, &task.description, offset)
    }
}

/// Central struct holding dispatcher configuration.
#[derive(Clone)]
pub struct ZbobrDispatcher {
    config: Arc<ZbobrDispatcherConfig>,
}

impl ZbobrDispatcher {
    /// Create a new Zbobr dispatcher from config.
    pub fn new(config: ZbobrDispatcherConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub fn config(&self) -> &ZbobrDispatcherConfig {
        &self.config
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_task(
        &self,
        backends: &Backends,
        title: &str,
        description: &str,
        stage: Stage,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
    ) -> anyhow::Result<u64> {
        self.create_task_with_confirm(
            backends,
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
        backends: &Backends,
        title: &str,
        description: &str,
        stage: Stage,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
        confirm: bool,
    ) -> anyhow::Result<u64> {
        let id = {
            let parameters = std::collections::HashMap::new();
            backends
                .tasks()
                .create_task(title, description, stage, parameters)
                .await?
        };
        // Set promoted fields + confirm flag via modify
        let weak = backends.tasks().get_task(id).await?;
        let mutable = weak.upgrade().await?;
        mutable
            .modify_task(Box::new(move |mut task| {
                task.destination_repository = destination_repository;
                task.destination_branch = destination_branch;
                if confirm {
                    task.confirm = true;
                }
                task
            }))
            .await?;
        Ok(id)
    }

    pub async fn setup_repository(&self, backends: &Backends, force: bool) -> anyhow::Result<()> {
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
        backends.tasks().setup(force).await
    }

    /// Extract repo name from a remote repo path (last path component).
    fn extract_repo_name(remote_repo: &str) -> &str {
        remote_repo.rsplit('/').next().unwrap_or(remote_repo)
    }

    /// Prepare a worktree for the given task. Constructs workspace_path as
    /// `TaskDir::new(workspaces, task_id)/repo_name` and delegates to the backend.
    pub async fn update_worktree(
        &self,
        backends: &Backends,
        identity: &zbobr_api::TaskIdentity,
    ) -> anyhow::Result<bool> {
        let repo_name = Self::extract_repo_name(&identity.destination_repository);
        let task_dir = TaskDir::new(&self.config.workspaces, identity.task_id);
        let workspace_path = task_dir.path().join(repo_name);
        backends
            .worktree()
            .update_worktree(identity, &workspace_path)
            .await
    }

    /// Create a TaskSession bound to a specific task (full dispatcher access).
    pub fn task_session(&self, backends: &Backends, task_id: u64) -> TaskSession {
        TaskSession::new(self.clone(), backends.clone(), task_id)
    }

    /// Create a RoleSession bound to a specific task (restricted MCP tool access).
    pub fn role_session(&self, backends: &Backends, task_id: u64) -> RoleSession {
        RoleSession::new(self.clone(), backends.clone(), task_id)
    }
}
