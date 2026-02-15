use std::path::Path;

use anyhow::Context;
use zbobr_backend_github::ZbobrBackendGithubToml;
use zbobr_dispatcher::ZbobrDispatcherToml;

/// Root TOML configuration for `zbobr`.
///
/// This wraps workspace-wide sections. The dispatcher-related options live
/// under the optional `dispatcher` table, and GitHub backend options under
/// `backend_github`. Example:
///
/// [dispatcher]
/// fork_owner = "myuser"
///
/// [backend_github]
/// task_repo = "owner/repo"
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default)]
pub struct ZbobrToml {
    pub dispatcher: Option<ZbobrDispatcherToml>,
    pub backend_github: Option<ZbobrBackendGithubToml>,
}

impl ZbobrToml {
    /// Load a root TOML file. Returns Ok(None) if the file does not exist.
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config: ZbobrToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {} as root zbobr TOML", path.display()))?;
        Ok(Some(config))
    }
}
