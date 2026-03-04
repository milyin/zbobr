use std::path::{Path, PathBuf};

use zbobr_api::task::Role;
use zbobr_utility::{config_struct, resolve_path};

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the mcp-tester executor.
pub struct ZbobrExecutorMcpTesterConfig {
    pub preparation: Option<PathBuf>,
    pub analysing: Option<PathBuf>,
    pub decompose_planning: Option<PathBuf>,
    pub decomposing: Option<PathBuf>,
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
            analysing: merged
                .analysing
                .as_ref()
                .map(|p| resolve_path(p.clone(), config_dir)),
            decompose_planning: merged
                .decompose_planning
                .as_ref()
                .map(|p| resolve_path(p.clone(), config_dir)),
            decomposing: merged
                .decomposing
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

    /// Get the scenario file path for the given role.
    pub fn scenario_for_role(&self, role: Role) -> Option<&PathBuf> {
        match role {
            Role::Preparator => self.preparation.as_ref(),
            Role::Analyser => self.analysing.as_ref(),
            Role::DecomposePlanner => self.decompose_planning.as_ref(),
            Role::Decomposer => self.decomposing.as_ref(),
            Role::Planner => self.planning.as_ref(),
            Role::Worker => self.working.as_ref(),
            Role::Reviewer => self.reviewing.as_ref(),
            Role::Tester => self.testing.as_ref(),
            Role::Merger => self.merging.as_ref(),
        }
    }
}
