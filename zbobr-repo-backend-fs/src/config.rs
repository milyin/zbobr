use std::path::PathBuf;

use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct(backend_config)]
/// Configuration for the filesystem repo backend.
pub struct ZbobrRepoBackendFs {
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
