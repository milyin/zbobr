use zbobr_dispatcher::task::Model;

/// TOML configuration for the Claude executor.
/// All fields are optional — missing fields fall back to defaults.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZbobrExecutorClaudeToml {
    pub default_model: Option<Model>,
}

/// Resolved configuration for the Claude executor.
#[derive(Debug, Clone)]
pub struct ZbobrExecutorClaudeConfig {
    /// Default AI model for Claude executor.
    pub default_model: Model,
}

impl Default for ZbobrExecutorClaudeConfig {
    fn default() -> Self {
        Self {
            default_model: Model::ClaudeOpus4_6,
        }
    }
}

impl ZbobrExecutorClaudeConfig {
    /// Build configuration by layering: defaults < TOML.
    pub fn build(toml: Option<&ZbobrExecutorClaudeToml>) -> Self {
        let defaults = Self::default();

        let default_model = toml
            .and_then(|t| t.default_model.clone())
            .unwrap_or(defaults.default_model);

        Self { default_model }
    }
}
