use zbobr_dispatcher::task::Model;
use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
pub struct ZbobrExecutorCopilot {
    /// Default AI model for Copilot executor.
    pub default_model: Model,
    /// GitHub token used by Copilot CLI (passed as COPILOT_GITHUB_TOKEN).
    pub copilot_github_token: String,
}

impl ZbobrExecutorCopilotConfig {
    /// Build configuration by layering: defaults < TOML < CLI args.
    pub fn build(toml: Option<ZbobrExecutorCopilotToml>, args: ZbobrExecutorCopilotArgs) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);

        let default_model = merged.default_model.unwrap_or(defaults.default_model);

        // token precedence: environment variables override TOML/CLI
        let copilot_github_token = std::env::var("COPILOT_GITHUB_TOKEN")
            .or_else(|_| std::env::var("GH_TOKEN"))
            .or_else(|_| std::env::var("GITHUB_TOKEN"))
            .unwrap_or_else(|_| {
                merged
                    .copilot_github_token
                    .unwrap_or(defaults.copilot_github_token.clone())
            });

        Self {
            default_model,
            copilot_github_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn build_uses_default_model_and_empty_token() {
        unsafe {
            env::remove_var("COPILOT_GITHUB_TOKEN");
            env::remove_var("GH_TOKEN");
            env::remove_var("GITHUB_TOKEN");
        }
        let cfg = ZbobrExecutorCopilotConfig::build(None, ZbobrExecutorCopilotArgs::default());
        assert_eq!(cfg.default_model, Model::default());
        assert!(cfg.copilot_github_token.is_empty());
    }

    #[test]
    fn build_prefers_env_var_over_toml() {
        // set a temporary env var
        unsafe {
            env::set_var("COPILOT_GITHUB_TOKEN", "from-env");
        }
        let toml = ZbobrExecutorCopilotToml {
            default_model: None,
            copilot_github_token: Some("from-toml".into()),
        };
        let cfg =
            ZbobrExecutorCopilotConfig::build(Some(toml), ZbobrExecutorCopilotArgs::default());
        assert_eq!(cfg.copilot_github_token, "from-env");
        unsafe {
            env::remove_var("COPILOT_GITHUB_TOKEN");
        }
    }

    #[test]
    fn build_falls_back_to_gh_token_env() {
        unsafe {
            env::remove_var("COPILOT_GITHUB_TOKEN");
        }
        unsafe {
            env::set_var("GH_TOKEN", "gh-env");
        }
        let cfg = ZbobrExecutorCopilotConfig::build(None, ZbobrExecutorCopilotArgs::default());
        assert_eq!(cfg.copilot_github_token, "gh-env");
        unsafe {
            env::remove_var("GH_TOKEN");
        }
    }
}
