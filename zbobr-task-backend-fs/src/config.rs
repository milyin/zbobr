use std::path::PathBuf;

use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct(backend_config)]
/// Configuration for the filesystem task backend.
pub struct ZbobrTaskBackendFs {
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
