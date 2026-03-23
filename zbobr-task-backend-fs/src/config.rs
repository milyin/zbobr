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

impl ZbobrTaskBackendFsConfig {
    /// Validate that all required fields are set.
    pub fn validate(&self) -> anyhow::Result<()> {
        // Tasks directory can be any path - we'll create it if it doesn't exist
        Ok(())
    }
}
