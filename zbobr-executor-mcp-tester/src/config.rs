use std::path::{Path, PathBuf};

use zbobr_utility::{config_struct, resolve_path};

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the mcp-tester executor.
///
/// Supports the legacy per-role fields (preparation, planning, etc.)
pub struct ZbobrExecutorMcpTesterConfig {
    pub preparation: Option<PathBuf>,
    pub planning: Option<PathBuf>,
    pub working: Option<PathBuf>,
    pub reviewing: Option<PathBuf>,
    pub testing: Option<PathBuf>,
    pub merging: Option<PathBuf>,
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
        Self {
            preparation: merged
                .preparation
                .as_ref()
                .map(|p| resolve_path(p.clone(), config_dir)),
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
        }
    }

    /// Get the scenario file path for the given stage name.
    /// Uses legacy per-role field mapping for backward compatibility.
    pub fn scenario_for_stage(&self, stage_name: &str) -> Option<&PathBuf> {
        match stage_name {
            "preparation" | "preparator" => self.preparation.as_ref(),
            "planning" | "planner" => self.planning.as_ref(),
            "working" | "worker" => self.working.as_ref(),
            "reviewing" | "reviewer" => self.reviewing.as_ref(),
            "testing" | "tester" => self.testing.as_ref(),
            "merging" | "merger" => self.merging.as_ref(),
            _ => None,
        }
    }
}
