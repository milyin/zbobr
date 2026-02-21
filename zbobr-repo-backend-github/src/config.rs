use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the GitHub repo backend.
pub struct ZbobrRepoBackendGithub {
    /// Owner for forks (GitHub user or org).
    #[arg(long)]
    pub fork_owner: String,
    /// GitHub token with read/write access to fork org.
    #[arg(
        long = "repo-github-token",
        env = "ZBOBR_REPO_GITHUB_TOKEN",
        id = "repo_github_token"
    )]
    pub github_token: String,
}

impl ZbobrRepoBackendGithubConfig {
    /// Build configuration by layering: defaults < TOML < args.
    pub(crate) fn build(
        toml: Option<ZbobrRepoBackendGithubToml>,
        args: ZbobrRepoBackendGithubArgs,
    ) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);

        let fork_owner = merged.fork_owner.unwrap_or(defaults.fork_owner);
        let github_token = merged.github_token.unwrap_or(defaults.github_token);

        Self {
            fork_owner,
            github_token,
        }
    }

    /// Validate that all required fields are set.
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        if self.fork_owner.is_empty() {
            anyhow::bail!(
                "fork owner not set. Use --repo-github-fork-owner NAME or set fork_owner in [repo.github] config.\n  \
                 This is the GitHub user or organization where target repos are forked for implementation."
            );
        }
        if self.github_token.is_empty() {
            anyhow::bail!(
                "GitHub token not set. Set github_token in [repo.github] config or use --repo-github-repo-github-token.\n  \
                 This token needs read/write access to the organization where repos are forked."
            );
        }
        Ok(())
    }
}
