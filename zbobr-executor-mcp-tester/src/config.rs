use std::path::{Path, PathBuf};

use zbobr_dispatcher::task::Role;
use zbobr_utility::{config_struct, resolve_path};

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the mcp-tester executor.
pub struct ZbobrExecutorMcpTester {
    #[arg(long = "executor-mcp-tester-preparation")]
    pub preparation: Option<PathBuf>,
    #[arg(long = "executor-mcp-tester-planning")]
    pub planning: Option<PathBuf>,
    #[arg(long = "executor-mcp-tester-working")]
    pub working: Option<PathBuf>,
    #[arg(long = "executor-mcp-tester-reviewing")]
    pub reviewing: Option<PathBuf>,
    #[arg(long = "executor-mcp-tester-merging")]
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
            Role::Planner => self.planning.as_ref(),
            Role::Worker => self.working.as_ref(),
            Role::Reviewer => self.reviewing.as_ref(),
            Role::Merger => self.merging.as_ref(),
        }
    }
}
