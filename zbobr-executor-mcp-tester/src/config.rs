use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use zbobr_utility::{config_struct, resolve_path};

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the mcp-tester executor.
///
/// Supports per-role fields (planning, working, etc.)
/// and a generic `scenarios` map for arbitrary role→scenario-file mappings.
pub struct ZbobrExecutorMcpTesterConfig {
    pub planning: Option<PathBuf>,
    pub working: Option<PathBuf>,
    pub reviewing: Option<PathBuf>,
    pub testing: Option<PathBuf>,
    pub merging: Option<PathBuf>,
    /// Generic role-name → scenario-file map (checked before legacy fields).
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
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k, resolve_path(v, config_dir)))
            .collect();
        Self {
            planning: merged
                .planning
                .as_ref()
                .map(|p| resolve_path(p.clone(), config_dir)),
            working: merged
                .working
                .as_ref()
                .map(|p| resolve_path(p.clone(), config_dir)),
            reviewing: merged
                .reviewing
                .as_ref()
                .map(|p| resolve_path(p.clone(), config_dir)),
            testing: merged
                .testing
                .as_ref()
                .map(|p| resolve_path(p.clone(), config_dir)),
            merging: merged
                .merging
                .as_ref()
                .map(|p| resolve_path(p.clone(), config_dir)),
            scenarios,
        }
    }

    /// Get the scenario file path for the given stage name.
    /// Checks the generic `scenarios` map first, then falls back to
    /// per-role field mapping.
    pub fn scenario_for_stage(&self, stage_name: &str) -> Option<&PathBuf> {
        if let Some(p) = self.scenarios.get(stage_name) {
            return Some(p);
        }
        match stage_name {
            "planning" | "planner" | "test_planner" => self.planning.as_ref(),
            "working" | "worker" | "test_worker" => self.working.as_ref(),
            "reviewing" | "reviewer" => self.reviewing.as_ref(),
            "testing" | "tester" => self.testing.as_ref(),
            "merging" | "merger" => self.merging.as_ref(),
            _ => None,
        }
    }
}
