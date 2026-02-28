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
