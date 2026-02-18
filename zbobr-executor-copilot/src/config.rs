use zbobr_dispatcher::task::Model;

/// TOML configuration for the Copilot executor.
/// All fields are optional — missing fields fall back to defaults.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZbobrExecutorCopilotToml {
    pub default_model: Option<Model>,
}

/// Resolved configuration for the Copilot executor.
#[derive(Debug, Clone)]
pub struct ZbobrExecutorCopilotConfig {
    /// Default AI model for Copilot executor.
    pub default_model: Model,
}

impl Default for ZbobrExecutorCopilotConfig {
    fn default() -> Self {
        Self {
            default_model: Model::Gpt5Mini,
        }
    }
}

impl ZbobrExecutorCopilotConfig {
    /// Build configuration by layering: defaults < TOML.
    pub fn build(toml: Option<&ZbobrExecutorCopilotToml>) -> Self {
        let defaults = Self::default();

        let default_model = toml
            .and_then(|t| t.default_model.clone())
            .unwrap_or(defaults.default_model);

        Self { default_model }
    }
}
