use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the GitHub task backend.
pub struct ZbobrTaskBackendGithubConfig {
    /// Task project repository ("Org/repo").
    #[arg(long)]
    pub github_repo: String,
    /// GitHub token with read/write access to tasks repo.
    #[arg(long, env = "ZBOBR_TASK_GITHUB_TOKEN")]
    pub github_token: String,
}

impl zbobr_api::config::BackendConfig for ZbobrTaskBackendGithubConfig {
    type Backend = crate::ArcTaskBackendGithub;

    fn build_backend(
        self,
        _dispatcher: &zbobr_api::config::ZbobrDispatcherConfig,
    ) -> anyhow::Result<Self::Backend> {
        let inner = crate::ZbobrTaskBackendGithub::from_config(self)?;
        Ok(crate::ArcTaskBackendGithub::new(inner))
    }
}

impl ZbobrTaskBackendGithubConfig {
    /// Validate that all required fields are set.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.github_repo.is_empty() {
            anyhow::bail!(
                "task repo not set. Use --tasks-github-repo owner/repo or set github_repo in the config file.\n  \
                 This is the GitHub repository whose issues the dispatcher processes."
            );
        }
        if self.github_token.is_empty() {
            anyhow::bail!(
                "GitHub token not set. Set github_token in [tasks.github] config or use --tasks-github-token.\n  \
                 This token needs read/write access to the tasks repo."
            );
        }
        Ok(())
    }

    /// Parse "owner/repo" into (owner, repo).
    pub fn parse_repo(&self) -> anyhow::Result<(&str, &str)> {
        let parts: Vec<&str> = self.github_repo.splitn(2, '/').collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid github_repo format '{}', expected 'owner/repo'",
                self.github_repo
            );
        }
        Ok((parts[0], parts[1]))
    }
}
