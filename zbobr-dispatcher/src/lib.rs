pub mod backend;
pub mod cleanup;
pub mod cli;
pub mod config;
pub mod mcp;
pub mod prompts;
pub mod setup;
pub mod state_machine;
pub mod task;
pub mod task_dir;
pub mod tool_executor;

pub use cli::{
    ConfigFileArg, ConfigLocation, GlobalArgs, parse_cli, print_task, process_task,
    resolve_config_location, run_manager_loop,
};
pub use config::{
    ZbobrDispatcherConfig, ZbobrDispatcherToml, ZbobrExecutorArgs, ZbobrExecutorToml,
};
pub use mcp::UnifiedMcp;
pub use prompts::{ConfiguredPromptBuilder, add_mcp_tool_variables, build_full_prompt, load_prompts, validate_stage_prompts};
pub use task::{
    ChecklistItem, Comment, Model, RoleSession, StackEntry, Task, TaskSession, Tool,
};
pub use task_dir::TaskDir;
pub use tool_executor::ToolExecutor;
pub use zbobr_api::config::Config;

use std::sync::Arc;

use typesafe_builder::{Builder, _TypesafeBuilderEmpty, _TypesafeBuilderFilled};
use zbobr_executor_claude::{ClaudeExecutor, ZbobrExecutorClaudeConfig};
use zbobr_executor_copilot::{CopilotExecutor, ZbobrExecutorCopilotConfig};
use zbobr_executor_mcp_tester::{McpTesterExecutor, ZbobrExecutorMcpTesterConfig};

use crate::backend::{TaskBackend, WorktreeBackend};

/// Central struct holding dispatcher configuration, backends, and executor settings.
#[derive(Builder)]
pub struct ZbobrDispatcher {
    #[builder(required)]
    config: Arc<ZbobrDispatcherConfig>,
    #[builder(required)]
    task_backend: Box<dyn TaskBackend>,
    #[builder(required)]
    repo_backend: Box<dyn WorktreeBackend>,
    #[builder(default = "Arc::new(ZbobrExecutorClaudeConfig::default())")]
    claude: Arc<ZbobrExecutorClaudeConfig>,
    #[builder(default = "Arc::new(ZbobrExecutorCopilotConfig::default())")]
    copilot: Arc<ZbobrExecutorCopilotConfig>,
    #[builder(default = "Arc::new(ZbobrExecutorMcpTesterConfig::default())")]
    mcp_tester: Arc<ZbobrExecutorMcpTesterConfig>,
    #[builder(optional)]
    prompt_builder: Option<ConfiguredPromptBuilder>,
}

impl ZbobrDispatcher {
    // -- Getters --

    pub fn config(&self) -> &ZbobrDispatcherConfig {
        &self.config
    }

    pub fn task_backend(&self) -> &dyn TaskBackend {
        &*self.task_backend
    }

    pub fn repo_backend(&self) -> &dyn WorktreeBackend {
        &*self.repo_backend
    }

    pub fn prompt_builder(&self) -> &ConfiguredPromptBuilder {
        self.prompt_builder.as_ref().expect("prompt_builder not set on ZbobrDispatcher")
    }

    pub fn mcp_tester_config(&self) -> &ZbobrExecutorMcpTesterConfig {
        &self.mcp_tester
    }

    pub fn build_executor(&self, tool: Tool, model: Model, mcp_tester_override: Option<&ZbobrExecutorMcpTesterConfig>) -> Box<dyn ToolExecutor> {
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
                config: mcp_tester_override.cloned().unwrap_or_else(|| self.mcp_tester.as_ref().clone()),
            }),
        }
    }

    pub fn copilot_github_token(&self) -> &str {
        &self.copilot.copilot_github_token
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_task(
        &self,
        title: &str,
        description: &str,
        state: &str,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
    ) -> anyhow::Result<u64> {
        self.create_task_with_confirm(
            title,
            description,
            state,
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
        state: &str,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
        confirm: bool,
    ) -> anyhow::Result<u64> {
        let id = self.task_backend.create_task(title, description, state).await?;
        // Set promoted fields + confirm flag via modify
        let weak = self.task_backend.get_task(id).await?;
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
        self.task_backend.setup(force).await
    }

    /// Extract repo name from a remote repo path (last path component).
    fn extract_repo_name(remote_repo: &str) -> &str {
        remote_repo.rsplit('/').next().unwrap_or(remote_repo)
    }

    /// Prepare a worktree for the given task. Constructs workspace_path as
    /// `TaskDir::new(workspaces, task_id)/repo_name` and delegates to the backend.
    pub async fn update_worktree(
        &self,
        identity: &zbobr_api::TaskIdentity,
    ) -> anyhow::Result<bool> {
        let repo_name = Self::extract_repo_name(&identity.destination_repository);
        let task_dir = TaskDir::new(&self.config.workspaces, identity.task_id);
        let workspace_path = task_dir.path().join(repo_name);
        self.repo_backend
            .update_worktree(identity, &workspace_path)
            .await
    }

    /// Create a TaskSession bound to a specific task (full dispatcher access).
    pub fn task_session(
        self: &Arc<Self>,
        task_id: u64,
    ) -> TaskSession {
        TaskSession::new(Arc::clone(self), task_id)
    }

    /// Create a RoleSession bound to a specific task (restricted MCP tool access).
    pub fn role_session(
        self: &Arc<Self>,
        task_id: u64,
    ) -> RoleSession {
        RoleSession::new(Arc::clone(self), task_id)
    }

    /// Create a RoleSession with a shared tool call tracker and comment buffer.
    pub fn role_session_with_tracker(
        self: &Arc<Self>,
        task_id: u64,
        tracker: Arc<std::sync::Mutex<Option<String>>>,
        comment_buffer: task::CommentBuffer,
    ) -> RoleSession {
        RoleSession::with_shared_tracker(Arc::clone(self), task_id, tracker, comment_buffer)
    }
}
