use std::path::Path;

use anyhow::Context;
use zbobr_dispatcher::ZbobrDispatcherToml;
use zbobr_executor_claude::ZbobrExecutorClaudeToml;
use zbobr_executor_copilot::ZbobrExecutorCopilotToml;
use zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterToml;
use zbobr_repo_backend_fs::ZbobrRepoBackendFsToml;
use zbobr_repo_backend_github::ZbobrRepoBackendGithubToml;
use zbobr_task_backend_fs::ZbobrTaskBackendFsToml;
use zbobr_task_backend_github::ZbobrTaskBackendGithubToml;

/// Task backend configuration section.
/// Each backend type has its own optional subsection.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default)]
pub struct ZbobrTaskBackendToml {
    pub github: Option<ZbobrTaskBackendGithubToml>,
    pub fs: Option<ZbobrTaskBackendFsToml>,
}

/// Repo backend configuration section.
/// Each backend type has its own optional subsection.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default)]
pub struct ZbobrRepoBackendToml {
    pub github: Option<ZbobrRepoBackendGithubToml>,
    pub fs: Option<ZbobrRepoBackendFsToml>,
}

/// Executor configuration section.
/// Each executor type has its own optional subsection.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default)]
pub struct ZbobrExecutorToml {
    pub claude: Option<ZbobrExecutorClaudeToml>,
    pub copilot: Option<ZbobrExecutorCopilotToml>,
    #[serde(rename = "mcp-tester")]
    pub mcp_tester: Option<ZbobrExecutorMcpTesterToml>,
}

/// Root TOML configuration for zbobr.
///
/// Example layout:
/// ```toml
/// [dispatcher]
/// default_model = "gpt-5-mini"
///
/// [task.github]
/// task_repo = "owner/repo"
///
/// [task.fs]
/// tasks_dir = "./tasks"
///
/// [repo.github]
/// fork_owner = "fork-owner"
///
/// [repo.fs]
/// repos_dir = "./repos"
///
/// [executor.claude]
/// default_model = "claude-opus-4.6"
///
/// [executor.copilot]
/// default_model = "gpt-5-mini"
///
/// [executor.mcp-tester]
/// planning = "scenarios/planning.yml"
/// working = "scenarios/working.yml"
/// ```
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default)]
pub struct ZbobrConfigToml {
    pub dispatcher: Option<ZbobrDispatcherToml>,
    pub task: Option<ZbobrTaskBackendToml>,
    pub repo: Option<ZbobrRepoBackendToml>,
    pub executor: Option<ZbobrExecutorToml>,
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
