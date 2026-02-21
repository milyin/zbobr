use zbobr_dispatcher::task::Model;
use zbobr_utility::config_struct;

#[config_struct]
pub struct ZbobrExecutorCopilot {
    /// Default AI model for Copilot executor.
    #[arg(
        env = "ZBOBR_EXECUTOR_COPILOT_DEFAULT_MODEL",
        help = "Default AI model for Copilot executor",
    )]
    pub default_model: Model,
}

/// Resolved configuration for the Copilot executor.
impl Clone for ZbobrExecutorCopilotConfig {
    fn clone(&self) -> Self {
        Self {
            default_model: self.default_model.clone(),
        }
    }
}

impl Default for ZbobrExecutorCopilotConfig {
    fn default() -> Self {
        Self {
            default_model: Model::Gpt5Mini,
        }
    }
}

impl ZbobrExecutorCopilotConfig {
    /// Build configuration by layering: defaults < TOML < CLI args.
    pub fn build(toml: Option<ZbobrExecutorCopilotToml>, args: ZbobrExecutorCopilotArgs) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);

        let default_model = merged.default_model.unwrap_or(defaults.default_model);

        Self { default_model }
    }
}
