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

/// Central struct holding configuration and backend.
#[derive(Clone)]
pub struct ZbobrDispatcher {
    config: Arc<ZbobrDispatcherConfig>,
    pub(crate) task_backend: Arc<dyn TaskBackend>,
    pub(crate) repo_backend: Arc<dyn WorktreeBackend>,
}

impl ZbobrDispatcher {
    /// Create a new Zbobr instance from config and pre-built backends.
    /// Used primarily in tests.
    pub fn new_with_backends(
        config: ZbobrDispatcherConfig,
        task_backend: Arc<dyn TaskBackend>,
        repo_backend: Arc<dyn WorktreeBackend>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            task_backend,
            repo_backend,
        }
    }

    pub fn config(&self) -> &ZbobrDispatcherConfig {
        &self.config
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
            self.tasks()
                .create_task(title, description, stage, parameters)
                .await?
        };
        if confirm {
            self.tasks()
                .modify_task(
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

    /// Fetch comments and description for a task, then extract the history chunk
    /// at the given `offset` using [`zbobr_api::extract_history_chunk`].
    pub async fn get_history(
        &self,
        id: u64,
        offset: Option<usize>,
    ) -> anyhow::Result<zbobr_api::HistoryChunk> {
        let comments = self.tasks().get_task_comments(id).await?;
        let desc = self
            .tasks()
            .get_task(id)
            .await
            .map(|t| t.description)
            .unwrap_or_default();
        zbobr_api::extract_history_chunk(comments, &desc, offset)
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
        self.tasks().setup(force).await
    }

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
        self.worktree()
            .update_worktree(remote_repo, base_branch, work_branch, &workspace_path)
            .await
    }

    pub fn debug_state(&self) -> String {
        format!(
            "task_backend: {}, repo_backend: {}",
            self.tasks().debug_state(),
            self.worktree().debug_state()
        )
    }

    /// Create a TaskSession bound to a specific task (full dispatcher access).
    pub fn task_session(&self, task_id: u64) -> TaskSession {
        TaskSession::new(self.clone(), task_id)
    }

    /// Create a RoleSession bound to a specific task (restricted MCP tool access).
    pub fn role_session(&self, task_id: u64) -> RoleSession {
        RoleSession::new(self.clone(), task_id)
    }
}
