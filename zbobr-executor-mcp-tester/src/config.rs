use std::path::{Path, PathBuf};

use zbobr_dispatcher::task::Role;
use zbobr_utility::{config_struct, resolve_path};

#[config_struct]
/// Configuration for the mcp-tester executor.
pub struct ZbobrExecutorMcpTester {
    #[arg(
        long = "executor-mcp-tester-preparation",
        env = "ZBOBR_EXECUTOR_MCP_TESTER_PREPARATION"
    )]
    pub preparation: Option<PathBuf>,
    #[arg(
        long = "executor-mcp-tester-planning",
        env = "ZBOBR_EXECUTOR_MCP_TESTER_PLANNING"
    )]
    pub planning: Option<PathBuf>,
    #[arg(
        long = "executor-mcp-tester-working",
        env = "ZBOBR_EXECUTOR_MCP_TESTER_WORKING"
    )]
    pub working: Option<PathBuf>,
    #[arg(
        long = "executor-mcp-tester-reviewing",
        env = "ZBOBR_EXECUTOR_MCP_TESTER_REVIEWING"
    )]
    pub reviewing: Option<PathBuf>,
    #[arg(
        long = "executor-mcp-tester-merging",
        env = "ZBOBR_EXECUTOR_MCP_TESTER_MERGING"
    )]
    pub merging: Option<PathBuf>,
}

/// Resolved configuration for the mcp-tester executor.
impl Clone for ZbobrExecutorMcpTesterConfig {
    fn clone(&self) -> Self {
        Self {
            preparation: self.preparation.clone(),
            planning: self.planning.clone(),
            working: self.working.clone(),
            reviewing: self.reviewing.clone(),
            merging: self.merging.clone(),
        }
    }
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
