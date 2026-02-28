use std::path::Path;

use anyhow::Context;
use zbobr_dispatcher::{
    ZbobrDispatcherConfig,
    config::{ZbobrDispatcherConfigArgs, ZbobrDispatcherConfigToml},
};
use zbobr_executor_claude::{
    ZbobrExecutorClaudeArgs, ZbobrExecutorClaudeConfig, ZbobrExecutorClaudeToml,
};
use zbobr_executor_copilot::{
    ZbobrExecutorCopilotArgs, ZbobrExecutorCopilotConfig, ZbobrExecutorCopilotToml,
};
use zbobr_executor_mcp_tester::{
    ZbobrExecutorMcpTesterArgs, ZbobrExecutorMcpTesterConfig, ZbobrExecutorMcpTesterToml,
};
use zbobr_repo_backend_fs::{
    ZbobrRepoBackendFsArgs, ZbobrRepoBackendFsConfig, ZbobrRepoBackendFsToml,
};
use zbobr_repo_backend_github::{
    ZbobrRepoBackendGithubArgs, ZbobrRepoBackendGithubConfig, ZbobrRepoBackendGithubToml,
};
use zbobr_task_backend_fs::{
    ZbobrTaskBackendFsArgs, ZbobrTaskBackendFsConfig, ZbobrTaskBackendFsToml,
};
use zbobr_task_backend_github::{
    ZbobrTaskBackendGithubArgs, ZbobrTaskBackendGithubConfig, ZbobrTaskBackendGithubToml,
};
use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Task backend configuration section.
pub struct ZbobrTaskBackendConfig {
    /// GitHub issues as the task source
    #[config(nested, toml_type = ZbobrTaskBackendGithubToml, args_type = ZbobrTaskBackendGithubArgs)]
    pub github: ZbobrTaskBackendGithubConfig,
    /// Filesystem task backend (YAML files in tasks/)
    #[config(nested, toml_type = ZbobrTaskBackendFsToml, args_type = ZbobrTaskBackendFsArgs)]
    pub fs: ZbobrTaskBackendFsConfig,
}

#[derive(Clone)]
#[config_struct]
/// Repo backend configuration section.
pub struct ZbobrRepoBackendConfig {
    /// GitHub repo backend (fork + push via API)
    #[config(nested, toml_type = ZbobrRepoBackendGithubToml, args_type = ZbobrRepoBackendGithubArgs)]
    pub github: ZbobrRepoBackendGithubConfig,
    /// Filesystem repo backend (operate on local clones)
    #[config(nested, toml_type = ZbobrRepoBackendFsToml, args_type = ZbobrRepoBackendFsArgs)]
    pub fs: ZbobrRepoBackendFsConfig,
}

#[derive(Clone)]
#[config_struct]
/// Executor configuration section.
pub struct ZbobrExecutorConfig {
    /// Claude-specific defaults
    #[config(nested, toml_type = ZbobrExecutorClaudeToml, args_type = ZbobrExecutorClaudeArgs)]
    pub claude: ZbobrExecutorClaudeConfig,
    /// GitHub Copilot executor defaults
    #[config(nested, toml_type = ZbobrExecutorCopilotToml, args_type = ZbobrExecutorCopilotArgs)]
    pub copilot: ZbobrExecutorCopilotConfig,
    /// MCP tester scenarios for validating MCP servers
    #[config(nested, toml_type = ZbobrExecutorMcpTesterToml, args_type = ZbobrExecutorMcpTesterArgs)]
    pub mcp_tester: ZbobrExecutorMcpTesterConfig,
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
    pub tasks: ZbobrTaskBackendConfig,
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
