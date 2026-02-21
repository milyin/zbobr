use std::path::{Path, PathBuf};

use zbobr_utility::{config_struct, resolve_path};

#[config_struct]
/// Configuration for the filesystem repo backend.
pub struct ZbobrRepoBackendFs {
    #[arg(long, env = "ZBOBR_REPOS_DIR")]
    pub repos_dir: PathBuf,
}

/// Resolved configuration for the filesystem repo backend.
#[derive(Debug, Clone)]
pub(crate) struct ZbobrRepoBackendFsRuntimeConfig {
    /// Base directory for repo operations and PR storage.
    pub(crate) repos_dir: PathBuf,
}

impl Default for ZbobrRepoBackendFsRuntimeConfig {
    fn default() -> Self {
        Self {
            repos_dir: PathBuf::from("./repos"),
        }
    }
}

impl ZbobrRepoBackendFsRuntimeConfig {
    /// Build configuration by layering: defaults < TOML < args.
    /// Relative paths from TOML are resolved against `config_dir`.
    pub(crate) fn build(
        toml: Option<ZbobrRepoBackendFsToml>,
        args: ZbobrRepoBackendFsArgs,
        config_dir: &Path,
    ) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);

        let repos_dir = merged
            .repos_dir
            .map(|p| resolve_path(p, config_dir))
            .unwrap_or(defaults.repos_dir);

        Self { repos_dir }
    }

    /// Validate that all required fields are set.
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        // repos_dir can be any path — we'll create it if needed
        Ok(())
    }
}
