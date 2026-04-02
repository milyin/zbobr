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

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

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
use zbobr_api::{
    State,
    config::{ResolvedProvider, ToolEntry},
};

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
    /// Providers excluded until the stored Instant (expiry).
    #[builder(default = "Arc::new(Mutex::new(HashMap::new()))")]
    excluded_providers: Arc<Mutex<HashMap<String, Instant>>>,
    /// Round-robin counters per tool name.
    #[builder(default = "Arc::new(Mutex::new(HashMap::new()))")]
    round_robin_state: Arc<Mutex<HashMap<String, usize>>>,
}

impl ZbobrDispatcher {
    /// Validate the dispatcher's own configuration and resolve all secrets.
    /// Chain after `build()` to ensure the config is consistent.
    pub fn validated(mut self) -> anyhow::Result<Self> {
        self.config.validate()?;
        self.config.validate_workflow_refs(self.workflow.config())?;
        // Eagerly resolve providers to catch circular inheritance at startup.
        self.config.resolve_providers()?;
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

    /// Select a provider and model for the given tool name.
    ///
    /// Filters out currently excluded providers, groups remaining entries by priority,
    /// and selects within the highest-priority group using round-robin.
    pub fn select_provider(&self, tool_name: &str) -> anyhow::Result<(ResolvedProvider, Model)> {
        let entries: &Vec<ToolEntry> = self
            .config
            .tools
            .get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found in dispatcher config", tool_name))?;

        let resolved_providers = self.config.resolve_providers()?;

        // Prune expired exclusions and snapshot current excluded set
        let now = Instant::now();
        let mut excluded_lock = self.excluded_providers.lock().unwrap();
        excluded_lock.retain(|_, expiry| *expiry > now);
        let excluded_set: HashSet<String> = excluded_lock.keys().cloned().collect();
        drop(excluded_lock);

        // Filter to available entries
        let available: Vec<(usize, &ToolEntry)> = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| !excluded_set.contains(&entry.provider))
            .collect();

        if available.is_empty() {
            anyhow::bail!(
                "All providers for tool '{}' are currently excluded",
                tool_name
            );
        }

        // Group by provider priority
        let mut priority_groups: HashMap<i32, Vec<(usize, &ToolEntry)>> = HashMap::new();
        for (idx, entry) in &available {
            let rp = resolved_providers
                .get(&entry.provider)
                .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", entry.provider))?;
            priority_groups
                .entry(rp.priority)
                .or_default()
                .push((*idx, *entry));
        }

        // Pick the highest-priority group
        let max_priority = priority_groups.keys().copied().max().unwrap();
        let top_group = &priority_groups[&max_priority];

        // Round-robin within the top group
        let mut rr = self.round_robin_state.lock().unwrap();
        let counter = rr.entry(tool_name.to_string()).or_insert(0);
        let pick = *counter % top_group.len();
        *counter = counter.wrapping_add(1);
        drop(rr);

        let (_, entry) = &top_group[pick];
        let rp = resolved_providers
            .get(&entry.provider)
            .unwrap()
            .clone();
        Ok((rp, entry.model.clone()))
    }

    /// Mark a provider as temporarily excluded.
    ///
    /// The provider will be skipped by `select_provider` until the exclusion expires.
    pub fn exclude_provider(&self, provider_name: &str) {
        let expiry = Instant::now()
            + Duration::from_secs(self.config.provider_exclusion_secs);
        self.excluded_providers
            .lock()
            .unwrap()
            .insert(provider_name.to_string(), expiry);
        tracing::info!(
            "Provider '{}' excluded for {}s",
            provider_name,
            self.config.provider_exclusion_secs
        );
    }

    /// Build an executor for the given resolved provider.
    pub fn build_executor(
        &self,
        provider: &ResolvedProvider,
        mcp_tester_override: Option<&ZbobrExecutorMcpTesterConfig>,
    ) -> anyhow::Result<Box<dyn ToolExecutor>> {
        match provider.executor.as_str() {
            "copilot" => Ok(Box::new(CopilotExecutor {
                config: self.copilot.config.clone(),
            })),
            "mcp-tester" => Ok(Box::new(McpTesterExecutor {
                config: mcp_tester_override
                    .cloned()
                    .unwrap_or_else(|| self.mcp_tester.config.clone()),
            })),
            "claude" => {
                let mut executor = ClaudeExecutor {
                    config: self.claude.config.clone(),
                    access_key: None,
                };
                if let Some(ref key) = provider.access_key {
                    executor.access_key = Some(key.clone());
                }
                Ok(Box::new(executor))
            }
            other => anyhow::bail!(
                "Unknown executor '{}' for provider '{}'",
                other,
                provider.name
            ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use zbobr_api::config::{ProviderDefinition, ToolEntry};

    // ── Mock backends ────────────────────────────────────────────────────

    struct MockTaskBackend;

    #[async_trait::async_trait]
    impl zbobr_api::backend::TaskBackend for MockTaskBackend {
        async fn get_task(&self, _id: u64) -> anyhow::Result<Box<dyn zbobr_api::backend::TaskWeak>> {
            unimplemented!()
        }
        async fn list_tasks(&self) -> anyhow::Result<Vec<Box<dyn zbobr_api::backend::TaskWeak>>> {
            unimplemented!()
        }
        async fn create_task(
            &self,
            _title: &str,
            _description: &str,
            _state: zbobr_api::State,
        ) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn setup(&self, _force: bool) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn validate_connectivity(&self) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn debug_state(&self) -> String {
            "mock".to_string()
        }
    }

    struct MockRepoBackend;

    #[async_trait::async_trait]
    impl zbobr_api::backend::WorktreeBackend for MockRepoBackend {
        fn repository(&self) -> &str { "mock/repo" }
        fn branch(&self) -> &str { "main" }
        fn repo_name(&self) -> &str { "repo" }
        async fn update_worktree(
            &self,
            _identity: &zbobr_api::TaskIdentity,
            _workspace_path: &std::path::Path,
            _git_user_name: &str,
            _git_user_email: &str,
        ) -> anyhow::Result<bool> {
            unimplemented!()
        }
        async fn fetch_refs(&self, _identity: &zbobr_api::TaskIdentity) -> anyhow::Result<()> {
            unimplemented!()
        }
        async fn ensure_pr_url(
            &self,
            _identity: &zbobr_api::TaskIdentity,
            _body: Option<&str>,
        ) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn validate_connectivity(&self) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn debug_state(&self) -> String {
            "mock".to_string()
        }
    }

    /// Build a minimal dispatcher with given providers and tools.
    fn make_dispatcher(
        providers: IndexMap<String, ProviderDefinition>,
        tools: IndexMap<String, Vec<ToolEntry>>,
    ) -> ZbobrDispatcher {
        let config = ZbobrDispatcherConfig {
            providers,
            tools,
            ..Default::default()
        };
        ZbobrDispatcherBuilder::new()
            .with_config(config)
            .with_workflow(Workflow::default())
            .with_task_backend(MockTaskBackend)
            .with_repo_backend(MockRepoBackend)
            .build()
    }

    fn provider_def(executor: &str, priority: i32) -> ProviderDefinition {
        ProviderDefinition {
            executor: Some(executor.to_string()),
            parent: None,
            priority: Some(priority),
            plan_mode: None,
            access_key: None,
        }
    }

    fn tool_entry(provider: &str, model: &str) -> ToolEntry {
        ToolEntry {
            provider: provider.to_string(),
            model: model.parse().unwrap(),
        }
    }

    // ── select_provider tests ────────────────────────────────────────────

    #[test]
    fn select_provider_basic() {
        let mut providers = IndexMap::new();
        providers.insert("claude".to_string(), provider_def("claude", 10));

        let mut tools = IndexMap::new();
        tools.insert("smart".to_string(), vec![tool_entry("claude", "opus")]);

        let dispatcher = make_dispatcher(providers, tools);
        let (rp, model) = dispatcher.select_provider("smart").unwrap();
        assert_eq!(rp.executor, "claude");
        assert_eq!(model.as_str(), "opus");
    }

    #[test]
    fn select_provider_prefers_higher_priority() {
        let mut providers = IndexMap::new();
        providers.insert("claude".to_string(), provider_def("claude", 10));
        providers.insert("fallback".to_string(), provider_def("claude", 0));

        let mut tools = IndexMap::new();
        tools.insert(
            "smart".to_string(),
            vec![
                tool_entry("fallback", "haiku"),
                tool_entry("claude", "opus"),
            ],
        );

        let dispatcher = make_dispatcher(providers, tools);
        let (rp, model) = dispatcher.select_provider("smart").unwrap();
        assert_eq!(rp.name, "claude");
        assert_eq!(model.as_str(), "opus");
    }

    #[test]
    fn select_provider_round_robin_same_priority() {
        let mut providers = IndexMap::new();
        providers.insert("a".to_string(), provider_def("claude", 10));
        providers.insert("b".to_string(), provider_def("copilot", 10));

        let mut tools = IndexMap::new();
        tools.insert(
            "smart".to_string(),
            vec![tool_entry("a", "model-a"), tool_entry("b", "model-b")],
        );

        let dispatcher = make_dispatcher(providers, tools);

        let (rp1, _) = dispatcher.select_provider("smart").unwrap();
        let (rp2, _) = dispatcher.select_provider("smart").unwrap();
        assert_ne!(rp1.name, rp2.name, "Round-robin should alternate providers");
    }

    #[test]
    fn select_provider_skips_excluded() {
        let mut providers = IndexMap::new();
        providers.insert("a".to_string(), provider_def("claude", 10));
        providers.insert("b".to_string(), provider_def("copilot", 10));

        let mut tools = IndexMap::new();
        tools.insert(
            "smart".to_string(),
            vec![tool_entry("a", "model-a"), tool_entry("b", "model-b")],
        );

        let dispatcher = make_dispatcher(providers, tools);
        dispatcher.exclude_provider("a");

        let (rp, _) = dispatcher.select_provider("smart").unwrap();
        assert_eq!(rp.name, "b");
    }

    #[test]
    fn select_provider_falls_back_to_lower_priority_when_higher_excluded() {
        let mut providers = IndexMap::new();
        providers.insert("primary".to_string(), provider_def("claude", 10));
        providers.insert("fallback".to_string(), provider_def("claude", 0));

        let mut tools = IndexMap::new();
        tools.insert(
            "smart".to_string(),
            vec![
                tool_entry("primary", "opus"),
                tool_entry("fallback", "haiku"),
            ],
        );

        let dispatcher = make_dispatcher(providers, tools);
        dispatcher.exclude_provider("primary");

        let (rp, model) = dispatcher.select_provider("smart").unwrap();
        assert_eq!(rp.name, "fallback");
        assert_eq!(model.as_str(), "haiku");
    }

    #[test]
    fn select_provider_all_excluded_error() {
        let mut providers = IndexMap::new();
        providers.insert("only".to_string(), provider_def("claude", 10));

        let mut tools = IndexMap::new();
        tools.insert("smart".to_string(), vec![tool_entry("only", "opus")]);

        let dispatcher = make_dispatcher(providers, tools);
        dispatcher.exclude_provider("only");

        let err = dispatcher.select_provider("smart").unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(msg.contains("excluded"), "Expected 'excluded' in error: {msg}");
    }

    #[test]
    fn select_provider_unknown_tool_error() {
        let dispatcher = make_dispatcher(IndexMap::new(), IndexMap::new());
        let err = dispatcher.select_provider("nonexistent").unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(msg.contains("not found"), "Expected 'not found' in error: {msg}");
    }

    // ── build_executor tests ─────────────────────────────────────────────

    #[test]
    fn build_executor_unknown_executor_error() {
        let dispatcher = make_dispatcher(IndexMap::new(), IndexMap::new());
        let bad_provider = zbobr_api::config::ResolvedProvider {
            name: "broken".to_string(),
            executor: "nonexistent".to_string(),
            priority: 10,
            plan_mode: false,
            access_key: None,
        };
        let result = dispatcher.build_executor(&bad_provider, None);
        assert!(result.is_err(), "Expected Err for unknown executor");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("Unknown executor"),
            "Expected 'Unknown executor' in error: {msg}"
        );
    }
}
