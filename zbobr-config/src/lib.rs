use std::path::Path;

use anyhow::Context;
use zbobr_dispatcher::{
    ZbobrDispatcherConfig,
    config::{ZbobrDispatcherConfigArgs, ZbobrDispatcherConfigToml},
};
use zbobr_executor_claude::{
    ZbobrExecutorClaudeArgs, ZbobrExecutorClaudeToml, config::ZbobrExecutorClaude,
};
use zbobr_executor_copilot::{
    ZbobrExecutorCopilotArgs, ZbobrExecutorCopilotToml, config::ZbobrExecutorCopilot,
};
use zbobr_executor_mcp_tester::{
    ZbobrExecutorMcpTesterArgs, ZbobrExecutorMcpTesterToml, config::ZbobrExecutorMcpTester,
};
use zbobr_repo_backend_fs::{
    ZbobrRepoBackendFsArgs, ZbobrRepoBackendFsToml, config::ZbobrRepoBackendFs,
};
use zbobr_repo_backend_github::{
    ZbobrRepoBackendGithubArgs, ZbobrRepoBackendGithubToml, config::ZbobrRepoBackendGithub,
};
use zbobr_task_backend_fs::{
    ZbobrTaskBackendFsArgs, ZbobrTaskBackendFsToml, config::ZbobrTaskBackendFs,
};
use zbobr_task_backend_github::{
    ZbobrTaskBackendGithubArgs, ZbobrTaskBackendGithubToml, config::ZbobrTaskBackendGithub,
};
use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Task backend configuration section.
pub struct ZbobrTaskBackendConfig {
    /// GitHub issues as the task source
    #[config(nested)]
    pub github: ZbobrTaskBackendGithub,
    /// Filesystem task backend (YAML files in tasks/)
    #[config(nested)]
    pub fs: ZbobrTaskBackendFs,
}

#[derive(Clone)]
#[config_struct]
/// Repo backend configuration section.
pub struct ZbobrRepoBackendConfig {
    /// GitHub repo backend (fork + push via API)
    #[config(nested)]
    pub github: ZbobrRepoBackendGithub,
    /// Filesystem repo backend (operate on local clones)
    #[config(nested)]
    pub fs: ZbobrRepoBackendFs,
}

#[derive(Clone)]
#[config_struct]
/// Executor configuration section.
pub struct ZbobrExecutorConfig {
    /// Claude-specific defaults
    #[config(nested)]
    pub claude: ZbobrExecutorClaude,
    /// GitHub Copilot executor defaults
    #[config(nested)]
    pub copilot: ZbobrExecutorCopilot,
    /// MCP tester scenarios for validating MCP servers
    #[config(nested)]
    pub mcp_tester: ZbobrExecutorMcpTester,
}

#[derive(Clone)]
#[config_struct]
/// Root configuration for zbobr.
pub struct ZbobrConfig {
    /// Dispatcher runtime: workspaces, prompts, tokens
    #[config(nested)]
    pub dispatcher: ZbobrDispatcherConfig,
    /// Task storage backends: control where zbobr discovers tasks.
    #[config(nested)]
    pub task: ZbobrTaskBackendConfig,
    /// Repo backends: where zbobr clones and pushes code.
    #[config(nested)]
    pub repo: ZbobrRepoBackendConfig,
    /// Executor defaults and scenarios.
    #[config(nested)]
    pub executor: ZbobrExecutorConfig,
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
