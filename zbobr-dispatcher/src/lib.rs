// Builder derive macro generates type params like ValueTask_backend from underscore field names
#![allow(non_camel_case_types)]

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
pub mod workflow;

use std::sync::Arc;

pub use cli::{
    ConfigFileArg, ConfigLocation, GlobalArgs, parse_cli, print_task, process_task,
    resolve_config_location, run_manager_loop,
};
pub use config::{
    ZbobrDispatcherConfig, ZbobrDispatcherToml, ZbobrExecutorArgs, ZbobrExecutorToml,
};
pub use mcp::UnifiedMcp;
pub use prompts::{
    ConfiguredPromptBuilder, VAR_DESTINATION_BRANCH, VAR_DESTINATION_REPOSITORY,
    add_mcp_tool_variables, build_full_prompt, load_prompts, validate_stage_prompts,
};
pub use task::{Comment, Model, RoleSession, StackEntry, Task, TaskSession, Tool};
pub use task_dir::TaskDir;
pub use tool_executor::ToolExecutor;
use typesafe_builder::{_TypesafeBuilderEmpty, _TypesafeBuilderFilled, Builder};
pub use workflow::{StateAction, Workflow};
use zbobr_api::State;

pub use zbobr_api::config::Config;
use zbobr_executor_claude::{ClaudeExecutor, ZbobrExecutorClaudeConfig};
use zbobr_executor_copilot::{CopilotExecutor, ZbobrExecutorCopilotConfig};
use zbobr_executor_mcp_tester::{McpTesterExecutor, ZbobrExecutorMcpTesterConfig};

use crate::backend::{TaskBackend, WorktreeBackend};

/// Central struct holding dispatcher configuration, backends, and executor settings.
#[derive(Builder)]
pub struct ZbobrDispatcher {
    #[builder(required)]
    config: ZbobrDispatcherConfig,
    #[builder(required)]
    workflow: Workflow,
    #[builder(required, into)]
    task_backend: Box<dyn TaskBackend>,
    #[builder(required, into)]
    repo_backend: Box<dyn WorktreeBackend>,
    #[builder(default = "ClaudeExecutor::new(ZbobrExecutorClaudeConfig::default())")]
    claude: ClaudeExecutor,
    #[builder(default = "CopilotExecutor::new(ZbobrExecutorCopilotConfig::default())")]
    copilot: CopilotExecutor,
    #[builder(default = "McpTesterExecutor::new(ZbobrExecutorMcpTesterConfig::default())")]
    mcp_tester: McpTesterExecutor,
    #[builder(optional)]
    prompt_builder: Option<ConfiguredPromptBuilder>,
}

impl ZbobrDispatcher {
    /// Validate the dispatcher's own configuration and resolve all secrets.
    /// Chain after `build()` to ensure the config is consistent.
    pub fn validated(mut self) -> anyhow::Result<Self> {
        self.config.validate()?;
        self.copilot.config.copilot_github_token.resolve()?;
        Ok(self)
    }

    // -- Getters --

    pub fn config(&self) -> &ZbobrDispatcherConfig {
        &self.config
    }

    pub fn workflow(&self) -> &Workflow {
        &self.workflow
    }

    pub fn task_backend(&self) -> &dyn TaskBackend {
        &*self.task_backend
    }

    pub fn repo_backend(&self) -> &dyn WorktreeBackend {
        &*self.repo_backend
    }

    pub fn prompt_builder(&self) -> &ConfiguredPromptBuilder {
        self.prompt_builder
            .as_ref()
            .expect("prompt_builder not set on ZbobrDispatcher")
    }

    pub fn mcp_tester_config(&self) -> &ZbobrExecutorMcpTesterConfig {
        &self.mcp_tester.config
    }

    pub fn build_executor(
        &self,
        tool: Tool,
        model: Model,
        mcp_tester_override: Option<&ZbobrExecutorMcpTesterConfig>,
    ) -> Box<dyn ToolExecutor> {
        match tool {
            Tool::Copilot => {
                let mut config = self.copilot.config.clone();
                config.default_model = model;
                Box::new(CopilotExecutor { config })
            }
            Tool::Claude => {
                let mut config = self.claude.config.clone();
                config.default_model = model;
                Box::new(ClaudeExecutor { config })
            }
            Tool::McpTester => Box::new(McpTesterExecutor {
                config: mcp_tester_override
                    .cloned()
                    .unwrap_or_else(|| self.mcp_tester.config.clone()),
            }),
        }
    }

    pub fn copilot_github_token(&self) -> &str {
        self.copilot.config.copilot_github_token.as_ref()
    }

    pub async fn create_task(
        &self,
        title: &str,
        description: &str,
        state: impl Into<State>,
    ) -> anyhow::Result<u64> {
        let state = state.into();
        self.create_task_with_confirm(title, description, state, false)
            .await
    }

    /// Like `create_task` but also set the confirm flag on the new task when
    /// `confirm == true`.
    pub async fn create_task_with_confirm(
        &self,
        title: &str,
        description: &str,
        state: impl Into<State>,
        confirm: bool,
    ) -> anyhow::Result<u64> {
        let id = self
            .task_backend
            .create_task(title, description, state.into())
            .await?;
        // Set confirm flag + max_stage_count via modify
        let max_stage_count = self.config.max_task_stage_count;
        let weak = self.task_backend.get_task(id).await?;
        let mutable = weak.upgrade().await?;
        mutable
            .modify_task(Box::new(move |mut task| {
                if confirm {
                    task.confirm = true;
                }
                task.max_stage_count = max_stage_count;
                task
            }))
            .await?;
        Ok(id)
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

    /// Fetch latest refs from origin with authentication.
    /// Unlike `update_worktree`, this only fetches — no merges, pushes, or PR side effects.
    pub async fn fetch_refs(&self, identity: &zbobr_api::TaskIdentity) -> anyhow::Result<()> {
        self.repo_backend.fetch_refs(identity).await
    }

    /// Prepare a worktree for the given task. Constructs workspace_path as
    /// `TaskDir::new(workspaces, task_id)/repo_name` and delegates to the backend.
    pub async fn update_worktree(
        &self,
        identity: &zbobr_api::TaskIdentity,
    ) -> anyhow::Result<bool> {
        let repo_name = self.repo_backend.repo_name();
        let task_dir = TaskDir::new(&self.config.workspaces, identity.task_id);
        let workspace_path = task_dir.path().join(repo_name);
        self.repo_backend
            .update_worktree(
                identity,
                &workspace_path,
                &self.config.git_user_name,
                &self.config.git_user_email,
            )
            .await
    }

    /// Create a TaskSession bound to a specific task (full dispatcher access).
    pub fn task_session(self: &Arc<Self>, task_id: u64) -> TaskSession {
        TaskSession::new(Arc::clone(self), task_id)
    }

    /// Create a RoleSession bound to a specific task (restricted MCP tool access).
    pub fn role_session(self: &Arc<Self>, task_id: u64) -> RoleSession {
        RoleSession::new(Arc::clone(self), task_id)
    }

    /// Create a RoleSession with a shared tool call tracker.
    pub fn role_session_with_tracker(
        self: &Arc<Self>,
        task_id: u64,
        tracker: Arc<std::sync::Mutex<Option<zbobr_api::config_tools::McpTool>>>,
        pipeline_name: String,
        pipeline_run_id: u64,
    ) -> RoleSession {
        RoleSession::with_shared_tracker(
            Arc::clone(self),
            task_id,
            tracker,
            pipeline_name,
            pipeline_run_id,
        )
    }
}
