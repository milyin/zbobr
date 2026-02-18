use std::path::{Path, PathBuf};

use zbobr_dispatcher::task::Role;

/// TOML configuration for the mcp-tester executor.
/// Maps each role to a YAML scenario file path.
///
/// Example:
/// ```toml
/// [executor.mcp-tester]
/// planning = "scenarios/planning.yml"
/// working = "scenarios/working.yml"
/// reviewing = "scenarios/reviewing.yml"
/// merging = "scenarios/merging.yml"
/// ```
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZbobrExecutorMcpTesterToml {
    pub planning: Option<PathBuf>,
    pub working: Option<PathBuf>,
    pub reviewing: Option<PathBuf>,
    pub merging: Option<PathBuf>,
}

/// Resolved configuration for the mcp-tester executor.
#[derive(Debug, Clone)]
pub struct ZbobrExecutorMcpTesterConfig {
    pub planning: Option<PathBuf>,
    pub working: Option<PathBuf>,
    pub reviewing: Option<PathBuf>,
    pub merging: Option<PathBuf>,
}

impl Default for ZbobrExecutorMcpTesterConfig {
    fn default() -> Self {
        Self {
            planning: None,
            working: None,
            reviewing: None,
            merging: None,
        }
    }
}

/// Resolve a path relative to `base_dir` if it is relative; leave absolute paths as-is.
fn resolve_path(path: Option<PathBuf>, base_dir: Option<&Path>) -> Option<PathBuf> {
    path.map(|p| {
        if p.is_relative() {
            if let Some(base) = base_dir {
                return base.join(&p);
            }
        }
        p
    })
}

impl ZbobrExecutorMcpTesterConfig {
    /// Build configuration by layering: defaults < TOML.
    /// Relative scenario paths are resolved against `config_dir` (the directory
    /// containing the config file).
    pub fn build(toml: Option<&ZbobrExecutorMcpTesterToml>, config_dir: Option<&Path>) -> Self {
        match toml {
            Some(t) => Self {
                planning: resolve_path(t.planning.clone(), config_dir),
                working: resolve_path(t.working.clone(), config_dir),
                reviewing: resolve_path(t.reviewing.clone(), config_dir),
                merging: resolve_path(t.merging.clone(), config_dir),
            },
            None => Self::default(),
        }
    }

    /// Get the scenario file path for the given role.
    pub fn scenario_for_role(&self, role: Role) -> Option<&PathBuf> {
        match role {
            Role::Planner => self.planning.as_ref(),
            Role::Worker => self.working.as_ref(),
            Role::Reviewer => self.reviewing.as_ref(),
            Role::Merger => self.merging.as_ref(),
        }
    }
}
