use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the GitHub repo backend.
pub struct ZbobrRepoBackendGithubConfig {
    /// Owner for forks (GitHub user or org).
    #[arg(long)]
    pub fork_owner: String,
    /// GitHub token with read/write access to fork org.
    #[arg(long, env = "ZBOBR_REPO_GITHUB_TOKEN")]
    pub token: String,
}

impl zbobr_api::config::BackendConfig for ZbobrRepoBackendGithubConfig {
    type Toml = ZbobrRepoBackendGithubToml;
    type Args = ZbobrRepoBackendGithubArgs;
    type Backend = crate::ZbobrRepoBackendGithub;

    fn build_config(
        toml: Option<Self::Toml>,
        args: Self::Args,
        _config_dir: &std::path::Path,
    ) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);
        Self {
            fork_owner: merged.fork_owner.unwrap_or(defaults.fork_owner),
            token: merged.token.unwrap_or(defaults.token),
        }
    }

    fn build_backend(
        self,
        dispatcher: &zbobr_api::config::ZbobrDispatcherConfig,
    ) -> anyhow::Result<Self::Backend> {
        crate::ZbobrRepoBackendGithub::from_config(
            self,
            dispatcher.git_user_name.clone(),
            dispatcher.git_user_email.clone(),
        )
    }
}

impl ZbobrRepoBackendGithubConfig {
    /// Validate that all required fields are set.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.fork_owner.is_empty() {
            anyhow::bail!(
                "fork owner not set. Use --repo-github-fork-owner NAME or set fork_owner in [repo.github] config.\n  \
                 This is the GitHub user or organization where target repos are forked for implementation."
            );
        }
        if self.token.is_empty() {
            anyhow::bail!(
                "GitHub token not set. Set token in [repo.github] config or use --repo-github-token.\n  \
                 This token needs read/write access to the organization where repos are forked."
            );
        }
        Ok(())
    }
}

impl zbobr_api::config::BackendWithConfig for crate::ZbobrRepoBackendGithub {
    type Config = ZbobrRepoBackendGithubConfig;
}
