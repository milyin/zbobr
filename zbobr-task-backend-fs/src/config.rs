use std::path::PathBuf;

use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Configuration for the filesystem task backend.
pub struct ZbobrTaskBackendFsConfig {
    /// Timezone for displaying comment timestamps.
    /// Injected from ZbobrDispatcherConfig; any TOML value is overwritten at runtime.
    #[config(skip_args)]
    pub timezone: Option<zbobr_api::task::FixedOffsetTz>,
    #[arg(long)]
    #[config(path)]
    pub tasks_dir: PathBuf,
    /// Optional default maximum number of stages for new tasks (0 disables limit).
    /// If absent, falls back to zbobr_api::task::DEFAULT_MAX_STAGE_COUNT.
    #[arg(long)]
    pub default_max_stage_count: u64,
}

impl Default for ZbobrTaskBackendFsConfig {
    fn default() -> Self {
        Self {
            timezone: None,
            tasks_dir: PathBuf::from("./tasks"),
            default_max_stage_count: zbobr_api::task::DEFAULT_MAX_STAGE_COUNT,
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
