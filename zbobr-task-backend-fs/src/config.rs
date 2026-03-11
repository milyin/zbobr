use std::path::PathBuf;

use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Configuration for the filesystem task backend.
pub struct ZbobrTaskBackendFsConfig {
    #[arg(long)]
    #[config(path)]
    pub tasks_dir: PathBuf,
}

impl Default for ZbobrTaskBackendFsConfig {
    fn default() -> Self {
        Self {
            tasks_dir: PathBuf::from("./tasks"),
        }
    }
}

impl zbobr_api::config::BackendConfig for ZbobrTaskBackendFsConfig {
    type Backend = crate::ArcTaskBackendFs;

    fn build_backend(
        self,
        _dispatcher: &zbobr_api::config::ZbobrDispatcherConfig,
    ) -> anyhow::Result<Self::Backend> {
        let inner = crate::ZbobrTaskBackendFs::from_config(self)?;
        Ok(crate::ArcTaskBackendFs::new(inner))
    }
}

impl ZbobrTaskBackendFsConfig {
    /// Validate that all required fields are set.
    pub fn validate(&self) -> anyhow::Result<()> {
        // Tasks directory can be any path - we'll create it if it doesn't exist
        Ok(())
    }
}
