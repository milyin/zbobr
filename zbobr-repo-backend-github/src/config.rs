use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the GitHub repo backend.
pub struct ZbobrRepoBackendGithub {
    /// Owner for forks (GitHub user or org).
    #[arg(long)]
    pub fork_owner: String,
    /// GitHub token with read/write access to fork org.
    #[arg(long, env = "ZBOBR_REPO_GITHUB_TOKEN")]
    pub token: String,
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
        let token = merged.token.unwrap_or(defaults.token);

        Self { fork_owner, token }
    }

    /// Validate that all required fields are set.
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
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
