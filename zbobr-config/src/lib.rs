use std::path::Path;

use anyhow::Context;
use zbobr_dispatcher::ZbobrDispatcherToml;
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
/// [repo.github]
/// fork_owner = "fork-owner"
/// ```
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default)]
pub struct ZbobrConfigToml {
    pub dispatcher: Option<ZbobrDispatcherToml>,
    pub task: Option<ZbobrTaskBackendToml>,
    pub repo: Option<ZbobrRepoBackendToml>,
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
