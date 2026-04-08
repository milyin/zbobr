use zbobr_api::Secret;
use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
pub struct ZbobrExecutorCopilotConfig {
    /// GitHub token used by Copilot CLI (passed as COPILOT_GITHUB_TOKEN).
    /// Use `{ value = "token" }` for an inline token or `{ env = "VAR" }` to read from an env var.
    #[config(skip_args)]
    pub copilot_github_token: Secret,
}

impl ZbobrExecutorCopilotConfig {
    /// Build configuration by layering: defaults < TOML < CLI args.
    pub fn build(toml: Option<ZbobrExecutorCopilotToml>, args: ZbobrExecutorCopilotArgs) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);

        let copilot_github_token = merged
            .copilot_github_token
            .into_option()
            .unwrap_or(defaults.copilot_github_token);

        Self {
            copilot_github_token,
        }
    }
}
