use std::path::PathBuf;

use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Configuration for the filesystem repo backend.
pub struct ZbobrRepoBackendFsConfig {
    #[arg(long)]
    pub repos_dir: PathBuf,
}

/// Resolved configuration for the filesystem repo backend.
impl Default for ZbobrRepoBackendFsConfig {
    fn default() -> Self {
        Self {
            repos_dir: PathBuf::from("./repos"),
        }
    }
}

impl zbobr_api::config::BackendConfig for ZbobrRepoBackendFsConfig {
    type Toml = ZbobrRepoBackendFsToml;
    type Args = ZbobrRepoBackendFsArgs;
    fn build_config(
        toml: Option<Self::Toml>,
        args: Self::Args,
        config_dir: &std::path::Path,
    ) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);
        Self {
            repos_dir: merged
                .repos_dir
                .map(|p| zbobr_utility::resolve_path(p, config_dir))
                .unwrap_or(defaults.repos_dir),
        }
    }
}

impl ZbobrRepoBackendFsConfig {
    /// Validate that all required fields are set.
    pub fn validate(&self) -> anyhow::Result<()> {
        // repos_dir can be any path — we'll create it if needed
        Ok(())
    }
}

impl zbobr_api::config::BuildableBackend for ZbobrRepoBackendFsConfig {
    type Backend = crate::ZbobrRepoBackendFs;

    fn build_backend(
        self,
        _dispatcher: &zbobr_api::config::ZbobrDispatcherConfig,
    ) -> anyhow::Result<Self::Backend> {
        crate::ZbobrRepoBackendFs::from_config(self)
    }
}

impl zbobr_api::config::BackendWithConfig for crate::ZbobrRepoBackendFs {
    type Config = ZbobrRepoBackendFsConfig;
}
