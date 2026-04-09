use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use zbobr_utility::{config_struct, resolve_path};

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the mcp-tester executor.
///
/// Uses a generic `scenarios` map for role/stage → scenario-file mappings.
pub struct ZbobrExecutorMcpTesterConfig {
    /// Generic stage-name → scenario-file map.
    #[config(skip_args)]
    pub scenarios: HashMap<String, PathBuf>,
}

impl ZbobrExecutorMcpTesterConfig {
    /// Build configuration by layering: defaults < TOML < args.
    /// Relative scenario paths are resolved against `config_dir`.
    pub fn build(
        toml: Option<ZbobrExecutorMcpTesterToml>,
        args: ZbobrExecutorMcpTesterArgs,
        config_dir: &Path,
    ) -> Self {
        let merged = toml.unwrap_or_default().merge_with_args(args);
        let scenarios = merged
            .scenarios
            .into_option()
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k, resolve_path(v, config_dir)))
            .collect();
        Self { scenarios }
    }

    /// Get the scenario file path for the given stage name.
    /// Checks the generic `scenarios` map first, then falls back to
    /// per-role field mapping.
    pub fn scenario_for_stage(&self, stage_name: &str) -> Option<&PathBuf> {
        self.scenarios.get(stage_name)
    }
}
