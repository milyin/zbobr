use std::path::PathBuf;
use zbobr_dispatcher::ZbobrError;

/// TOML configuration for the filesystem task backend.
/// All fields are optional — missing fields fall back to defaults.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZbobrTaskBackendFsToml {
    pub tasks_dir: Option<String>,
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
    /// Build configuration by layering: defaults < env < TOML < overrides.
    pub(crate) fn build(
        toml: Option<&ZbobrTaskBackendFsToml>,
        tasks_dir_override: Option<&str>,
    ) -> Self {
        let defaults = Self::default();

        let tasks_dir = tasks_dir_override
            .map(PathBuf::from)
            .or_else(|| toml.and_then(|t| t.tasks_dir.clone()).map(PathBuf::from))
            .or_else(|| std::env::var("ZBOBR_TASKS_DIR").ok().map(PathBuf::from))
            .unwrap_or(defaults.tasks_dir);

        Self { tasks_dir }
    }

    /// Validate that all required fields are set.
    pub(crate) fn validate(&self) -> Result<(), ZbobrError> {
        // Tasks directory can be any path - we'll create it if it doesn't exist
        Ok(())
    }
}
