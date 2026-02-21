use zbobr_dispatcher::task::Model;
use zbobr_utility::config_struct;

#[config_struct]
pub struct ZbobrExecutorClaude {
    /// Default AI model for Claude executor.
    #[arg(
        env = "ZBOBR_EXECUTOR_CLAUDE_DEFAULT_MODEL",
        help = "Default AI model for Claude executor",
    )]
    pub default_model: Model,
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
    /// Build configuration by layering: defaults < TOML < CLI args.
    pub fn build(toml: Option<ZbobrExecutorClaudeToml>, args: ZbobrExecutorClaudeArgs) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);

        let default_model = merged.default_model.unwrap_or(defaults.default_model);

        Self { default_model }
    }
}
