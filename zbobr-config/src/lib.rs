use std::path::Path;

use anyhow::Context;
use zbobr_backend_github::ZbobrBackendGithubToml;
use zbobr_dispatcher::ZbobrDispatcherToml;

/// Backend configuration section.
/// Each backend type has its own optional subsection.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default)]
pub struct ZbobrBackendToml {
    pub github: Option<ZbobrBackendGithubToml>,
}

/// Root TOML configuration for zbobr.
///
/// Example layout:
/// ```toml
/// [dispatcher]
/// default_model = "gpt-5-mini"
///
/// [backend.github]
/// task_repo = "owner/repo"
/// ```
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default)]
pub struct ZbobrConfigToml {
    pub dispatcher: Option<ZbobrDispatcherToml>,
    pub backend: Option<ZbobrBackendToml>,
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
