use std::path::PathBuf;

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

impl ZbobrExecutorMcpTesterConfig {
    /// Build configuration by layering: defaults < TOML.
    pub fn build(toml: Option<&ZbobrExecutorMcpTesterToml>) -> Self {
        match toml {
            Some(t) => Self {
                planning: t.planning.clone(),
                working: t.working.clone(),
                reviewing: t.reviewing.clone(),
                merging: t.merging.clone(),
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
