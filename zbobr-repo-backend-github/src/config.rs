use zbobr_dispatcher::ZbobrError;

/// TOML configuration for the GitHub repo backend.
/// All fields are optional — missing fields fall back to defaults.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZbobrRepoBackendGithubToml {
    pub fork_owner: Option<String>,
    pub github_token: Option<String>,
}

/// Resolved configuration for the GitHub repo backend.
#[derive(Debug, Clone, Default)]
pub(crate) struct ZbobrRepoBackendGithubConfig {
    /// Owner for forks (GitHub user or org).
    pub(crate) fork_owner: String,
    /// GitHub token with read/write access to fork org.
    pub(crate) github_token: String,
}

impl ZbobrRepoBackendGithubConfig {
    /// Build configuration by layering: defaults < env < TOML < overrides.
    pub(crate) fn build(
        toml: Option<&ZbobrRepoBackendGithubToml>,
        fork_owner_override: Option<&str>,
    ) -> Self {
        let defaults = Self::default();

        let fork_owner = fork_owner_override
            .map(String::from)
            .or_else(|| toml.and_then(|t| t.fork_owner.clone()))
            .unwrap_or(defaults.fork_owner);

        // github_token: GH_TOKEN > GITHUB_TOKEN > TOML
        let github_token = std::env::var("GH_TOKEN")
            .ok()
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
            .or_else(|| toml.and_then(|t| t.github_token.clone()))
            .unwrap_or(defaults.github_token);

        Self {
            fork_owner,
            github_token,
        }
    }

    /// Validate that all required fields are set.
    pub(crate) fn validate(&self) -> Result<(), ZbobrError> {
        if self.fork_owner.is_empty() {
            return Err(ZbobrError::Config(
                "fork owner not set. Use --fork-owner NAME or set fork_owner in [repo.github] config.\n  \
                 This is the GitHub user or organization where target repos are forked for implementation."
                    .into(),
            ));
        }
        if self.github_token.is_empty() {
            return Err(ZbobrError::Config(
                "GitHub token not set. Set GH_TOKEN or GITHUB_TOKEN env var, or set github_token in [repo.github] config.\n  \
                 This token needs read/write access to the organization where repos are forked."
                    .into(),
            ));
        }
        Ok(())
    }
}
