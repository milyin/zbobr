use super::*;
use crate::Zbobr;
use crate::task::{Model, Tool};

struct StubTaskBackend;

#[async_trait::async_trait]
impl crate::backend::TaskBackend for StubTaskBackend {
    async fn get_task(&self, _id: u64) -> anyhow::Result<crate::Task> {
        unimplemented!()
    }
    async fn create_task(
        &self,
        _title: &str,
        _description: &str,
        _stage: crate::Stage,
        _tool: Option<crate::Tool>,
        _model: Option<crate::Model>,
        _parameters: std::collections::HashMap<crate::Parameter, String>,
    ) -> anyhow::Result<u64> {
        unimplemented!()
    }
    async fn close_task(&self, _id: u64) -> anyhow::Result<()> {
        unimplemented!()
    }
    async fn is_task_closed(&self, _id: u64) -> anyhow::Result<bool> {
        unimplemented!()
    }
    async fn modify_task(
        &self,
        _id: u64,
        _mutate: Box<dyn FnOnce(crate::Task) -> crate::Task + Send>,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
    async fn list_tasks_by_stage(
        &self,
        _stage: crate::Stage,
        _tool: Option<crate::Tool>,
    ) -> anyhow::Result<Vec<crate::Task>> {
        unimplemented!()
    }
    async fn get_task_comments(&self, _id: u64) -> anyhow::Result<Vec<String>> {
        unimplemented!()
    }
    async fn post_task_comment(
        &self,
        _id: u64,
        _body: &str,
        _role: &str,
        _hostname: &str,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
    async fn setup(&self, _force: bool) -> anyhow::Result<()> {
        unimplemented!()
    }
    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn debug_state(&self) -> String {
        "StubTaskBackend".to_string()
    }
}

struct StubRepoBackend;

#[async_trait::async_trait]
impl crate::backend::RepoBackend for StubRepoBackend {
    async fn clone_and_setup(
        &self,
        _repo: &str,
        _branch: &str,
        _workspace_path: &std::path::Path,
    ) -> anyhow::Result<std::path::PathBuf> {
        unimplemented!()
    }
    async fn clone_readonly(
        &self,
        _repo: &str,
        _branch: &str,
        _workspace_path: &std::path::Path,
    ) -> anyhow::Result<std::path::PathBuf> {
        unimplemented!()
    }
    async fn sync_fork(&self, _repo: &str, _branch: &str) -> anyhow::Result<()> {
        unimplemented!()
    }
    async fn setup_fork_remote_and_push(
        &self,
        _work_dir: &std::path::Path,
        _target_repo: &str,
        _work_branch: &str,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
    async fn push_and_create_pr(
        &self,
        _repo: &str,
        _workspace_path: &std::path::Path,
        _pr_title: &str,
        _pr_body: &str,
    ) -> anyhow::Result<String> {
        unimplemented!()
    }
    async fn create_pr_in_fork(
        &self,
        _repo_name: &str,
        _work_branch: &str,
        _dest_branch: &str,
        _pr_title: &str,
        _pr_body: &str,
    ) -> anyhow::Result<String> {
        unimplemented!()
    }
    async fn parse_pr_to_repo_branch(&self, _pr_ref: &str) -> anyhow::Result<(String, String)> {
        unimplemented!()
    }
    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn debug_state(&self) -> String {
        "StubRepoBackend".to_string()
    }
}

fn test_config() -> crate::config::ZbobrDispatcherConfig {
    crate::config::ZbobrDispatcherConfig {
        workspaces: std::path::PathBuf::from("/tmp"),
        agent_github_token: "agent-token".to_string(),
        backend: crate::config::BackendType::GitHub,
        cli_tool: Tool::Claude,
        preparator_prompts: vec![],
        planner_prompts: vec![],
        worker_prompts: vec![],
        reviewer_prompts: vec![],
        merger_prompts: vec![],
        work_branch_prefix: "zbobr_fix".to_string(),
        prompts_path: None,
        git_user_name: "Test User".to_string(),
        git_user_email: "test@example.com".to_string(),
    }
}
