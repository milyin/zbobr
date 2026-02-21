use std::path::Path;

use anyhow::Context;
use zbobr_dispatcher::{ZbobrDispatcherArgs, ZbobrDispatcherToml};
use zbobr_executor_claude::{ZbobrExecutorClaudeArgs, ZbobrExecutorClaudeToml};
use zbobr_executor_copilot::{ZbobrExecutorCopilotArgs, ZbobrExecutorCopilotToml};
use zbobr_executor_mcp_tester::{
    ZbobrExecutorMcpTesterArgs, ZbobrExecutorMcpTesterToml,
};
use zbobr_repo_backend_fs::{ZbobrRepoBackendFsArgs, ZbobrRepoBackendFsToml};
use zbobr_repo_backend_github::{
    ZbobrRepoBackendGithubArgs, ZbobrRepoBackendGithubToml,
};
use zbobr_task_backend_fs::{ZbobrTaskBackendFsArgs, ZbobrTaskBackendFsToml};
use zbobr_task_backend_github::{ZbobrTaskBackendGithubArgs, ZbobrTaskBackendGithubToml};
use zbobr_utility::config_struct;

#[config_struct]
/// Task backend configuration section.
pub struct ZbobrTaskBackend {
    /// GitHub issues as the task source
    #[config(
        nested,
        heading_prefix = "task-github",
        args_type = ZbobrTaskBackendGithubArgs,
        toml_type = ZbobrTaskBackendGithubToml,
        help_heading = "GitHub issues as the task source",
    )]
    pub github: ZbobrTaskBackendGithubArgs,
    /// Filesystem task backend (YAML files in tasks/)
    #[config(
        nested,
        heading_prefix = "task-fs",
        args_type = ZbobrTaskBackendFsArgs,
        toml_type = ZbobrTaskBackendFsToml,
        help_heading = "Filesystem task backend (YAML files in tasks/)",
    )]
    pub fs: ZbobrTaskBackendFsArgs,
}

#[config_struct]
/// Repo backend configuration section.
pub struct ZbobrRepoBackend {
    /// GitHub repo backend (fork + push via API)
    #[config(
        nested,
        heading_prefix = "repo-github",
        args_type = ZbobrRepoBackendGithubArgs,
        toml_type = ZbobrRepoBackendGithubToml,
        help_heading = "GitHub repo backend (fork + push via API)",
    )]
    pub github: ZbobrRepoBackendGithubArgs,
    /// Filesystem repo backend (operate on local clones)
    #[config(
        nested,
        heading_prefix = "repo-fs",
        args_type = ZbobrRepoBackendFsArgs,
        toml_type = ZbobrRepoBackendFsToml,
        help_heading = "Filesystem repo backend (operate on local clones)",
    )]
    pub fs: ZbobrRepoBackendFsArgs,
}

#[config_struct]
/// Executor configuration section.
pub struct ZbobrExecutor {
    /// Claude-specific defaults
    #[config(
        nested,
        heading_prefix = "executor-claude",
        args_type = ZbobrExecutorClaudeArgs,
        toml_type = ZbobrExecutorClaudeToml,
        help_heading = "Claude-specific defaults",
    )]
    pub claude: ZbobrExecutorClaudeArgs,
    /// GitHub Copilot executor defaults
    #[config(
        nested,
        heading_prefix = "executor-copilot",
        args_type = ZbobrExecutorCopilotArgs,
        toml_type = ZbobrExecutorCopilotToml,
        help_heading = "GitHub Copilot executor defaults",
    )]
    pub copilot: ZbobrExecutorCopilotArgs,
    /// MCP tester scenarios for validating MCP servers
    #[config(
        nested,
        heading_prefix = "executor-mcp-tester",
        args_type = ZbobrExecutorMcpTesterArgs,
        toml_type = ZbobrExecutorMcpTesterToml,
        toml_rename = "mcp-tester",
        help_heading = "MCP tester scenarios for validating MCP servers",
    )]
    pub mcp_tester: ZbobrExecutorMcpTesterArgs,
}

#[config_struct]
/// Root configuration for zbobr.
pub struct ZbobrConfig {
    /// Dispatcher runtime: workspaces, prompts, tokens
    #[config(
        nested,
        heading_prefix = "dispatcher",
        args_type = ZbobrDispatcherArgs,
        toml_type = ZbobrDispatcherToml,
        help_heading = "Dispatcher runtime: workspaces, prompts, tokens",
    )]
    pub dispatcher: ZbobrDispatcherArgs,
    /// Task storage backends: control where zbobr discovers tasks.
    #[config(
        nested,
        heading_prefix = "task",
        args_type = ZbobrTaskBackendArgs,
        toml_type = ZbobrTaskBackendToml,
        help_heading = "Task storage backends: control where zbobr discovers tasks.",
    )]
    pub task: ZbobrTaskBackendArgs,
    /// Repo backends: where zbobr clones and pushes code.
    #[config(
        nested,
        heading_prefix = "repo",
        args_type = ZbobrRepoBackendArgs,
        toml_type = ZbobrRepoBackendToml,
        help_heading = "Repo backends: where zbobr clones and pushes code.",
    )]
    pub repo: ZbobrRepoBackendArgs,
    /// Executor defaults and scenarios.
    #[config(
        nested,
        heading_prefix = "executor",
        args_type = ZbobrExecutorArgs,
        toml_type = ZbobrExecutorToml,
        help_heading = "Executor defaults and scenarios.",
    )]
    pub executor: ZbobrExecutorArgs,
}

impl ZbobrConfigToml {
    /// Load a TOML config from a file path.
    /// Returns Ok(None) if the file does not exist.
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config: ZbobrConfigToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(Some(config))
    }
}
