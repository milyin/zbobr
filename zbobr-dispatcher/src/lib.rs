pub mod backend;
pub mod cleanup;
pub mod config;
pub mod mcp;
pub mod setup;
pub mod task;
pub mod tool_executor;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

pub use config::{ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml};
pub use mcp::{
    MergerMcp, PlannerMcp, PreparatorMcp, ReviewerMcp, WorkerMcp, merger_instructions,
    planner_instructions, preparator_instructions, reviewer_instructions, worker_instructions,
};
pub use task::{ChecklistItem, Model, Parameter, Signal, Stage, Task, TaskSession, Tool};
pub use tool_executor::ToolExecutor;

use crate::backend::{RepoBackend, TaskBackend};

/// Central struct holding configuration and backend.
#[derive(Clone)]
pub struct Zbobr {
    config: Arc<ZbobrDispatcherConfig>,
    pub(crate) task_backend: Arc<dyn TaskBackend>,
    pub(crate) repo_backend: Arc<dyn RepoBackend>,
    /// Per-task mutexes to serialize concurrent read-modify-write cycles
    /// for the same task within this process.
    task_locks: Arc<std::sync::Mutex<HashMap<u64, Arc<tokio::sync::Mutex<()>>>>>,
}

impl Zbobr {
    /// Create a new Zbobr instance from config and pre-built backends.
    pub fn new(
        config: ZbobrDispatcherConfig,
        task_backend: Arc<dyn TaskBackend>,
        repo_backend: Arc<dyn RepoBackend>,
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

    /// Create a TaskSession bound to a specific task.
    pub fn task_session(&self, task_id: u64) -> TaskSession {
        TaskSession::new(self.clone(), task_id)
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
        tool: Option<Tool>,
        model: Option<Model>,
        destination_repository: Option<String>,
        destination_branch: Option<String>,
    ) -> anyhow::Result<u64> {
        let mut parameters = std::collections::HashMap::new();
        if let Some(repo) = destination_repository {
            parameters.insert(Parameter::DestinationRepository, repo);
        }
        if let Some(branch) = destination_branch {
            parameters.insert(Parameter::DestinationBranch, branch);
        }

        self.task_backend
            .create_task(title, description, stage, tool, model, parameters)
            .await
    }

    pub async fn close_task(&self, id: u64) -> anyhow::Result<()> {
        self.task_backend.close_task(id).await
    }

    pub async fn get_task_comments(&self, id: u64) -> anyhow::Result<Vec<String>> {
        self.task_backend.get_task_comments(id).await
    }

    pub async fn post_task_comment(
        &self,
        id: u64,
        body: &str,
        role: &str,
        hostname: &str,
    ) -> anyhow::Result<()> {
        self.task_backend
            .post_task_comment(id, body, role, hostname)
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

    pub async fn list_tasks_by_stage(
        &self,
        stage: Stage,
        tool: Option<Tool>,
    ) -> anyhow::Result<Vec<Task>> {
        self.task_backend.list_tasks_by_stage(stage, tool).await
    }

    pub async fn is_task_closed(&self, id: u64) -> anyhow::Result<bool> {
        self.task_backend.is_task_closed(id).await
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

    // -- Delegate to RepoBackend --

    pub async fn clone_and_setup(
        &self,
        target_repo: &str,
        branch: &str,
        task_id: u64,
    ) -> anyhow::Result<PathBuf> {
        let workspace_path = self.config.workspaces.join(format!("task#{task_id}"));
        self.repo_backend
            .clone_and_setup(target_repo, branch, &workspace_path)
            .await
    }

    pub async fn clone_readonly(
        &self,
        target_repo: &str,
        branch: &str,
        task_id: u64,
    ) -> anyhow::Result<PathBuf> {
        let workspace_path = self.config.workspaces.join(format!("task#{task_id}"));
        self.repo_backend
            .clone_readonly(target_repo, branch, &workspace_path)
            .await
    }

    /// Parse PR reference to (repo, branch).
    /// Accepts formats:
    /// - "https://github.com/owner/repo/pull/123"
    /// - "owner/repo#123"
    ///   Returns (repo, branch_name)
    pub async fn parse_pr_to_repo_branch(&self, pr_ref: &str) -> anyhow::Result<(String, String)> {
        self.repo_backend.parse_pr_to_repo_branch(pr_ref).await
    }

    pub async fn push_and_create_pr(
        &self,
        target_repo: &str,
        task_id: u64,
        pr_title: &str,
        pr_body: &str,
    ) -> anyhow::Result<String> {
        let workspace_path = self.config.workspaces.join(format!("task#{task_id}"));
        self.repo_backend
            .push_and_create_pr(target_repo, &workspace_path, pr_title, pr_body)
            .await
    }

    pub async fn create_pr_in_fork(
        &self,
        repo_name: &str,
        work_branch: &str,
        destination_branch: &str,
        pr_title: &str,
        pr_body: &str,
    ) -> anyhow::Result<String> {
        self.repo_backend
            .create_pr_in_fork(
                repo_name,
                work_branch,
                destination_branch,
                pr_title,
                pr_body,
            )
            .await
    }

    pub async fn setup_fork_remote_and_push(
        &self,
        work_dir: &std::path::Path,
        target_repo: &str,
        work_branch: &str,
    ) -> anyhow::Result<()> {
        self.repo_backend
            .setup_fork_remote_and_push(work_dir, target_repo, work_branch)
            .await
    }

    /// Ensure the fork is synchronized with the upstream `target_repo` on `branch`.
    pub async fn sync_fork(&self, target_repo: &str, branch: &str) -> anyhow::Result<()> {
        self.repo_backend.sync_fork(target_repo, branch).await
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
