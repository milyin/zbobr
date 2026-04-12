use zbobr_api::Secret;
use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
pub struct ZbobrExecutorCopilotConfig {
    /// GitHub token used by Copilot CLI (passed as COPILOT_GITHUB_TOKEN).
    /// Use `{ value = "token" }` for an inline token or `{ env = "VAR" }` to read from an env var.
    /// When absent, zbobr falls back to `gh auth token` at execution time.
    #[config(skip_args)]
    pub copilot_github_token: Option<Secret>,
}

impl ZbobrExecutorCopilotConfig {
    /// Build configuration by layering: defaults < TOML < CLI args.
    pub fn build(toml: Option<ZbobrExecutorCopilotToml>, args: ZbobrExecutorCopilotArgs) -> Self {
        let merged = toml.unwrap_or_default().merge_with_args(args);
        Self {
            copilot_github_token: merged.copilot_github_token.into_option(),
        }
    }
}
