use zbobr_utility::config_struct;

/// Configuration for the Claude executor.
///
/// The model and access_key are provided at runtime via provider definitions —
/// there are no static TOML fields for this executor.
#[derive(Clone, Default)]
#[config_struct]
pub struct ZbobrExecutorClaudeConfig {}

impl ZbobrExecutorClaudeConfig {
    /// Build configuration — no TOML fields to merge.
    pub fn build(_toml: Option<ZbobrExecutorClaudeToml>, _args: ZbobrExecutorClaudeArgs) -> Self {
        Self::default()
    }
}
