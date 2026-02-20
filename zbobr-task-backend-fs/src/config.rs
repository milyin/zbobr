use std::path::{Path, PathBuf};
use zbobr_utility::{config_struct, resolve_path};

config_struct! {
    /// Configuration for the filesystem task backend.
    pub struct ZbobrTaskBackendFs {
        #[arg(long, env = "ZBOBR_TASKS_DIR")]
        pub tasks_dir: PathBuf,
    }
}

/// Resolved configuration for the filesystem task backend.
#[derive(Debug, Clone)]
pub(crate) struct ZbobrTaskBackendFsConfig {
    /// Directory where tasks are stored as YAML files.
    pub(crate) tasks_dir: PathBuf,
}

impl Default for ZbobrTaskBackendFsConfig {
    fn default() -> Self {
        Self {
            tasks_dir: PathBuf::from("./tasks"),
        }
    }
}

impl ZbobrTaskBackendFsConfig {
    /// Build configuration by layering: defaults < TOML < args.
    /// Relative paths from TOML are resolved against `config_dir`.
    pub(crate) fn build(
        toml: Option<ZbobrTaskBackendFsToml>,
        args: ZbobrTaskBackendFsArgs,
        config_dir: &Path,
    ) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);

        let tasks_dir = merged
            .tasks_dir
            .map(|p| resolve_path(p, config_dir))
            .unwrap_or(defaults.tasks_dir);

        Self { tasks_dir }
    }

    /// Validate that all required fields are set.
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        // Tasks directory can be any path - we'll create it if it doesn't exist
        Ok(())
    }
}
