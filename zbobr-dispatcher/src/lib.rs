pub mod backend;
pub mod cleanup;
pub mod config;
pub mod mcp;
pub mod setup;
pub mod task;
pub mod tool_executor;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

pub use config::{ZbobrDispatcherToml, ZbobrDispatcherConfig};
pub use mcp::{planner_instructions, worker_instructions, reviewer_instructions, merger_instructions, PlannerMcp, WorkerMcp, ReviewerMcp, MergerMcp};
pub use task::{Model, ChecklistItem, Parameter, Signal, Stage, Task, TaskSession, Tool};
pub use tool_executor::{ClaudeExecutor, CopilotExecutor, ToolExecutor};

use crate::backend::Backend;

/// Central struct holding configuration and backend.
#[derive(Clone)]
pub struct Zbobr {
    config: Arc<ZbobrDispatcherConfig>,
    pub(crate) backend: Arc<dyn Backend>,
    /// Per-task mutexes to serialize concurrent read-modify-write cycles
    /// for the same task within this process.
    task_locks: Arc<std::sync::Mutex<HashMap<u64, Arc<tokio::sync::Mutex<()>>>>>,
}

impl Zbobr {
    /// Create a new Zbobr instance from config and a pre-built backend.
    pub fn new(config: ZbobrDispatcherConfig, backend: Arc<dyn Backend>) -> Self {
        Self {
            config: Arc::new(config),
            backend,
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

    /// Validate that the backend can reach required resources (fork owner, task repo, etc.).
    pub async fn validate_connectivity(&self) -> Result<(), ZbobrError> {
        self.backend.validate_connectivity().await
    }

    // -- Delegate to Backend --

    pub async fn get_task(&self, id: u64) -> Result<Task, ZbobrError> {
        self.backend.get_task(id).await
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
    ) -> Result<u64, ZbobrError> {
        let mut parameters = std::collections::HashMap::new();
        if let Some(repo) = destination_repository {
            parameters.insert(Parameter::DestinationRepository, repo);
        }
        if let Some(branch) = destination_branch {
            parameters.insert(Parameter::DestinationBranch, branch);
        }
        
        self.backend.create_task(title, description, stage, tool, model, parameters).await
    }

    pub async fn close_task(&self, id: u64) -> Result<(), ZbobrError> {
        self.backend.close_task(id).await
    }

    pub async fn get_task_comments(&self, id: u64) -> Result<Vec<String>, ZbobrError> {
        self.backend.get_task_comments(id).await
    }

    pub async fn post_task_comment(
        &self,
        id: u64,
        body: &str,
        role: &str,
        hostname: &str,
    ) -> Result<(), ZbobrError> {
        self.backend
            .post_task_comment(id, body, role, hostname)
            .await
    }

    pub async fn set_task_stage(
        &self,
        id: u64,
        stage: Stage,
    ) -> Result<(), ZbobrError> {
        self.backend.set_task_stage(id, stage.milestone_name()).await
    }

    pub async fn set_task_signal(&self, id: u64, signal: Option<Signal>) -> Result<(), ZbobrError> {
        self.backend.set_task_signal(id, signal).await
    }

    pub async fn update_task_description(
        &self,
        id: u64,
        description: &str,
    ) -> Result<(), ZbobrError> {
        self.backend.update_task_description(id, description).await
    }

    pub async fn list_tasks_by_stage(
        &self,
        stage_name: &str,
        tool: Option<Tool>,
    ) -> Result<Vec<Task>, ZbobrError> {
        self.backend.list_tasks_by_stage(stage_name, tool).await
    }

    pub async fn is_task_closed(&self, id: u64) -> Result<bool, ZbobrError> {
        self.backend.is_task_closed(id).await
    }

    pub async fn repo_file_exists(&self, path: &str) -> Result<bool, ZbobrError> {
        self.backend.repo_file_exists(path).await
    }

    pub async fn create_repo_file(
        &self,
        path: &str,
        content: &str,
        commit_message: &str,
    ) -> Result<(), ZbobrError> {
        self.backend
            .create_repo_file(path, content, commit_message)
            .await
    }

    pub async fn setup_repository(&self, force: bool) -> Result<(), ZbobrError> {
        self.backend.setup_repository(force).await
    }

    pub async fn ensure_task_repo_exists(&self) -> Result<(), ZbobrError> {
        self.backend.ensure_task_repo_exists().await
    }

    pub async fn clone_and_setup(
        &self,
        target_repo: &str,
        branch: &str,
        task_id: u64,
    ) -> Result<PathBuf, ZbobrError> {
        self.backend
            .clone_and_setup(target_repo, branch, task_id)
            .await
    }

    pub async fn clone_readonly(
        &self,
        target_repo: &str,
        branch: &str,
        task_id: u64,
    ) -> Result<PathBuf, ZbobrError> {
        self.backend
            .clone_readonly(target_repo, branch, task_id)
            .await
    }

    /// Parse PR reference to (repo, branch).
    /// Accepts formats:
    /// - "https://github.com/owner/repo/pull/123"
    /// - "owner/repo#123"
    ///   Returns (repo, branch_name)
    pub async fn parse_pr_to_repo_branch(
        &self,
        pr_ref: &str,
    ) -> Result<(String, String), ZbobrError> {
        self.backend.parse_pr_to_repo_branch(pr_ref).await
    }

    pub async fn push_and_create_pr(
        &self,
        target_repo: &str,
        task_id: u64,
    ) -> Result<String, ZbobrError> {
        self.backend.push_and_create_pr(target_repo, task_id).await
    }

    pub async fn create_pr_in_fork(
        &self,
        repo_name: &str,
        work_branch: &str,
        destination_branch: &str,
        task_id: u64,
    ) -> Result<String, ZbobrError> {
        self.backend
            .create_pr_in_fork(repo_name, work_branch, destination_branch, task_id)
            .await
    }

    pub async fn setup_fork_remote_and_push(
        &self,
        work_dir: &std::path::Path,
        target_repo: &str,
        work_branch: &str,
    ) -> Result<(), ZbobrError> {
        self.backend
            .setup_fork_remote_and_push(work_dir, target_repo, work_branch)
            .await
    }

    /// Ensure the fork is synchronized with the upstream `target_repo` on `branch`.
    pub async fn sync_fork(&self, target_repo: &str, branch: &str) -> Result<(), ZbobrError> {
        self.backend.sync_fork(target_repo, branch).await
    }

    pub async fn list_stages(&self) -> Result<Vec<(u64, String)>, ZbobrError> {
        self.backend.list_stages().await
    }

    pub async fn create_stage(&self, title: &str, description: &str) -> Result<(), ZbobrError> {
        self.backend.create_stage(title, description).await
    }

    pub async fn delete_stage(&self, number: u64) -> Result<(), ZbobrError> {
        self.backend.delete_stage(number).await
    }

    pub async fn list_labels(&self) -> Result<Vec<String>, ZbobrError> {
        self.backend.list_labels().await
    }

    pub async fn create_label(
        &self,
        name: &str,
        color: &str,
        description: &str,
    ) -> Result<(), ZbobrError> {
        self.backend.create_label(name, color, description).await
    }

    pub fn debug_state(&self) -> String {
        self.backend.debug_state()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ZbobrError {
    #[error("GitHub API error: {0}")]
    GitHub(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

