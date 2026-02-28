use std::path::PathBuf;

use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Configuration for the filesystem task backend.
pub struct ZbobrTaskBackendFsConfig {
    #[arg(long)]
    pub tasks_dir: PathBuf,
}

/// Resolved configuration for the filesystem task backend.
impl Default for ZbobrTaskBackendFsConfig {
    fn default() -> Self {
        Self {
            tasks_dir: PathBuf::from("./tasks"),
        }
    }
}

impl zbobr_api::config::BackendConfig for ZbobrTaskBackendFsConfig {
    type Toml = ZbobrTaskBackendFsToml;
    type Args = ZbobrTaskBackendFsArgs;
    fn build_config(
        toml: Option<Self::Toml>,
        args: Self::Args,
        config_dir: &std::path::Path,
    ) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);
        Self {
            tasks_dir: merged
                .tasks_dir
                .map(|p| zbobr_utility::resolve_path(p, config_dir))
                .unwrap_or(defaults.tasks_dir),
        }
    }
}

impl ZbobrTaskBackendFsConfig {
    /// Validate that all required fields are set.
    pub fn validate(&self) -> anyhow::Result<()> {
        // Tasks directory can be any path - we'll create it if it doesn't exist
        Ok(())
    }
}

impl zbobr_api::config::BuildableBackend for ZbobrTaskBackendFsConfig {
    type Backend = crate::ZbobrTaskBackendFs;

    fn build_backend(
        self,
        _dispatcher: &zbobr_api::config::ZbobrDispatcherConfig,
    ) -> anyhow::Result<Self::Backend> {
        crate::ZbobrTaskBackendFs::from_config(self)
    }
}

impl zbobr_api::config::BackendWithConfig for crate::ZbobrTaskBackendFs {
    type Config = ZbobrTaskBackendFsConfig;
}
