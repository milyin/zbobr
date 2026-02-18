use std::path::{Path, PathBuf};

use zbobr_utility::resolve_path;

/// TOML configuration for the filesystem repo backend.
/// All fields are optional — missing fields fall back to defaults.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZbobrRepoBackendFsToml {
    pub repos_dir: Option<PathBuf>,
}

/// Resolved configuration for the filesystem repo backend.
#[derive(Debug, Clone)]
pub(crate) struct ZbobrRepoBackendFsConfig {
    /// Base directory for repo operations and PR storage.
    pub(crate) repos_dir: PathBuf,
}

impl Default for ZbobrRepoBackendFsConfig {
    fn default() -> Self {
        Self {
            repos_dir: PathBuf::from("./repos"),
        }
    }
}

impl ZbobrRepoBackendFsConfig {
    /// Build configuration by layering: defaults < env < TOML < overrides.
    /// Relative paths from TOML are resolved against `config_dir`.
    pub(crate) fn build(
        toml: Option<&ZbobrRepoBackendFsToml>,
        repos_dir_override: Option<&str>,
        config_dir: &Path,
    ) -> Self {
        let defaults = Self::default();

        let repos_dir = repos_dir_override
            .map(PathBuf::from)
            .or_else(|| {
                toml.and_then(|t| t.repos_dir.clone())
                    .map(|p| resolve_path(p, config_dir))
            })
            .or_else(|| std::env::var("ZBOBR_REPOS_DIR").ok().map(PathBuf::from))
            .unwrap_or(defaults.repos_dir);

        Self { repos_dir }
    }

    /// Validate that all required fields are set.
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        // repos_dir can be any path — we'll create it if needed
        Ok(())
    }
}
