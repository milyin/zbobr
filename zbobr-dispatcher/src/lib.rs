pub mod backend;
pub mod cleanup;
pub mod cli;
pub mod config;
pub mod mcp;
pub mod prompts;
pub mod setup;
pub mod task;
pub mod task_dir;
pub mod tool_executor;

pub use cli::{
    Command, ConfigFileArg, ConfigLocation, GlobalArgs, TaskSubcommand, parse_cli, print_task,
    resolve_config_location,
};
pub use config::{
    ZbobrDispatcherConfig, ZbobrDispatcherToml, ZbobrExecutorArgs, ZbobrExecutorToml,
};
pub use mcp::{MergerMcp, PlannerMcp, PreparatorMcp, ReviewerMcp, TesterMcp, WorkerMcp};
pub use prompts::{
    ConfiguredPromptBuilder, PromptsConfig, build_full_prompt, build_prompt_for_role, load_prompts,
    validate_prompts,
};
pub use task::{
    ChecklistItem, Comment, CommentType, Model, RoleSession, Signal, Stage, Task, TaskSession, Tool,
};
pub use task_dir::TaskDir;
pub use tool_executor::ToolExecutor;
pub use zbobr_api::config::Config;

use std::sync::Arc;

use zbobr_executor_claude::{ClaudeExecutor, ZbobrExecutorClaudeConfig};
use zbobr_executor_copilot::{CopilotExecutor, ZbobrExecutorCopilotConfig};
use zbobr_executor_mcp_tester::{McpTesterExecutor, ZbobrExecutorMcpTesterConfig};

use crate::backend::{TaskBackend, WorktreeBackend};

/// Fetch comments and description for a task, then extract the history chunk
/// at the given `offset` using [`zbobr_api::extract_history_chunk`].
pub async fn get_history(
    task_backend: &dyn TaskBackend,
    id: u64,
    offset: Option<usize>,
) -> anyhow::Result<zbobr_api::HistoryChunk> {
    let weak = task_backend.get_task(id).await?;
    let comments = weak.get_comments().await?;
    let task = weak.snapshot().await?;
    zbobr_api::extract_history_chunk(comments, &task.description, offset)
}

/// Central struct holding dispatcher configuration, backends, and executor settings.
#[derive(Clone)]
pub struct ZbobrDispatcher {
    config: Arc<ZbobrDispatcherConfig>,
    claude: Arc<ZbobrExecutorClaudeConfig>,
    copilot: Arc<ZbobrExecutorCopilotConfig>,
    mcp_tester: Arc<ZbobrExecutorMcpTesterConfig>,
    task_backend: Option<Arc<dyn TaskBackend>>,
    repo_backend: Option<Arc<dyn WorktreeBackend>>,
    prompt_builder: Option<ConfiguredPromptBuilder>,
}

impl ZbobrDispatcher {
    /// Create a new Zbobr dispatcher from config with default executor settings.
    pub fn new(config: ZbobrDispatcherConfig) -> Self {
        Self::new_with_executors(
            config,
            ZbobrExecutorClaudeConfig::default(),
            ZbobrExecutorCopilotConfig::default(),
            ZbobrExecutorMcpTesterConfig::default(),
        )
    }

    /// Create a new Zbobr dispatcher from dispatcher and executor configs.
    pub fn new_with_executors(
        config: ZbobrDispatcherConfig,
        claude: ZbobrExecutorClaudeConfig,
        copilot: ZbobrExecutorCopilotConfig,
        mcp_tester: ZbobrExecutorMcpTesterConfig,
    ) -> Self {
        Self {
            config: Arc::new(config),
            claude: Arc::new(claude),
            copilot: Arc::new(copilot),
            mcp_tester: Arc::new(mcp_tester),
            task_backend: None,
            repo_backend: None,
            prompt_builder: None,
        }
    }

    /// Clone dispatcher with an overridden MCP tester executor config.
    ///
    /// This is used for one-off scenario overrides (for example from CLI flags)
    /// while keeping all other executor settings unchanged.
    pub fn with_mcp_tester_config(&self, mcp_tester: ZbobrExecutorMcpTesterConfig) -> Self {
        Self {
            config: Arc::clone(&self.config),
            claude: Arc::clone(&self.claude),
            copilot: Arc::clone(&self.copilot),
            mcp_tester: Arc::new(mcp_tester),
            task_backend: self.task_backend.clone(),
            repo_backend: self.repo_backend.clone(),
            prompt_builder: self.prompt_builder.clone(),
        }
    }

    // -- Setters --

    pub fn set_task_backend(&mut self, backend: Arc<dyn TaskBackend>) {
        self.task_backend = Some(backend);
    }

    pub fn set_repo_backend(&mut self, backend: Arc<dyn WorktreeBackend>) {
        self.repo_backend = Some(backend);
    }

    pub fn set_prompt_builder(&mut self, builder: ConfiguredPromptBuilder) {
        self.prompt_builder = Some(builder);
    }

    pub fn set_claude_config(&mut self, config: ZbobrExecutorClaudeConfig) {
        self.claude = Arc::new(config);
    }

    pub fn set_copilot_config(&mut self, config: ZbobrExecutorCopilotConfig) {
        self.copilot = Arc::new(config);
    }

    pub fn set_mcp_tester_config(&mut self, config: ZbobrExecutorMcpTesterConfig) {
        self.mcp_tester = Arc::new(config);
    }

    // -- Getters --

    pub fn config(&self) -> &ZbobrDispatcherConfig {
        &self.config
    }

    pub fn task_backend(&self) -> &Arc<dyn TaskBackend> {
        self.task_backend.as_ref().expect("task_backend not set on ZbobrDispatcher")
    }

    pub fn repo_backend(&self) -> &Arc<dyn WorktreeBackend> {
        self.repo_backend.as_ref().expect("repo_backend not set on ZbobrDispatcher")
    }

    pub fn prompt_builder(&self) -> &ConfiguredPromptBuilder {
        self.prompt_builder.as_ref().expect("prompt_builder not set on ZbobrDispatcher")
    }

    pub(crate) fn build_executor(&self, tool: Tool, model: Model) -> Box<dyn ToolExecutor> {
        match tool {
            Tool::Copilot => {
                let mut config = self.copilot.as_ref().clone();
                config.default_model = model;
                Box::new(CopilotExecutor { config })
            }
            Tool::Claude => {
                let mut config = self.claude.as_ref().clone();
                config.default_model = model;
                Box::new(ClaudeExecutor { config })
            }
            Tool::McpTester => Box::new(McpTesterExecutor {
                config: self.mcp_tester.as_ref().clone(),
            }),
        }
    }

    pub(crate) fn copilot_github_token(&self) -> &str {
        &self.copilot.copilot_github_token
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_task(
        &self,
        task_backend: &dyn TaskBackend,
        title: &str,
        description: &str,
        stage: Stage,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
    ) -> anyhow::Result<u64> {
        self.create_task_with_confirm(
            task_backend,
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
        task_backend: &dyn TaskBackend,
        title: &str,
        description: &str,
        stage: Stage,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
        confirm: bool,
    ) -> anyhow::Result<u64> {
        let id = task_backend.create_task(title, description, stage).await?;
        // Set promoted fields + confirm flag via modify
        let weak = task_backend.get_task(id).await?;
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

    pub async fn setup_repository(
        &self,
        task_backend: &dyn TaskBackend,
        force: bool,
    ) -> anyhow::Result<()> {
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
        task_backend.setup(force).await
    }

    /// Extract repo name from a remote repo path (last path component).
    fn extract_repo_name(remote_repo: &str) -> &str {
        remote_repo.rsplit('/').next().unwrap_or(remote_repo)
    }

    /// Prepare a worktree for the given task. Constructs workspace_path as
    /// `TaskDir::new(workspaces, task_id)/repo_name` and delegates to the backend.
    pub async fn update_worktree(
        &self,
        repo_backend: &dyn WorktreeBackend,
        identity: &zbobr_api::TaskIdentity,
    ) -> anyhow::Result<bool> {
        let repo_name = Self::extract_repo_name(&identity.destination_repository);
        let task_dir = TaskDir::new(&self.config.workspaces, identity.task_id);
        let workspace_path = task_dir.path().join(repo_name);
        repo_backend
            .update_worktree(identity, &workspace_path)
            .await
    }

    /// Create a TaskSession bound to a specific task (full dispatcher access).
    pub fn task_session(
        &self,
        task_backend: Arc<dyn TaskBackend>,
        repo_backend: Arc<dyn WorktreeBackend>,
        task_id: u64,
    ) -> TaskSession {
        TaskSession::new(self.clone(), task_backend, repo_backend, task_id)
    }

    /// Create a RoleSession bound to a specific task (restricted MCP tool access).
    pub fn role_session(
        &self,
        task_backend: Arc<dyn TaskBackend>,
        task_id: u64,
    ) -> RoleSession {
        RoleSession::new(self.clone(), task_backend, task_id)
    }
}
