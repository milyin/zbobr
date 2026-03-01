use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the GitHub repo backend.
pub struct ZbobrRepoBackendGithubConfig {
    /// Owner for forks (GitHub user or org).
    #[arg(long)]
    pub fork_owner: String,
    /// GitHub token with read/write access to fork org.
    #[arg(name = "repo-github-token", long = "repo-github-token", env = "ZBOBR_REPO_GITHUB_TOKEN")]
    pub token: String,
}

impl zbobr_api::config::BackendConfig for ZbobrRepoBackendGithubConfig {
    type Backend = crate::ZbobrRepoBackendGithub;

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
