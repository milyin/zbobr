use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the GitHub task backend.
pub struct ZbobrTaskBackendGithubConfig {
    /// Task project repository ("Org/repo").
    #[arg(long)]
    pub task_repo: String,
    /// GitHub token with read/write access to tasks repo.
    #[arg(long, env = "ZBOBR_TASK_GITHUB_TOKEN")]
    pub token: String,
}

impl zbobr_api::config::BackendConfig for ZbobrTaskBackendGithubConfig {
    type Toml = ZbobrTaskBackendGithubToml;
    type Args = ZbobrTaskBackendGithubArgs;
    type Backend = crate::ZbobrTaskBackendGithub;

    fn build_config(
        toml: Option<Self::Toml>,
        args: Self::Args,
        _config_dir: &std::path::Path,
    ) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);
        Self {
            task_repo: merged.task_repo.unwrap_or(defaults.task_repo),
            token: merged.token.unwrap_or(defaults.token),
        }
    }

    fn build_backend(
        self,
        _dispatcher: &zbobr_api::config::ZbobrDispatcherConfig,
    ) -> anyhow::Result<Self::Backend> {
        crate::ZbobrTaskBackendGithub::from_config(self)
    }
}

impl ZbobrTaskBackendGithubConfig {
    /// Validate that all required fields are set.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.task_repo.is_empty() {
            anyhow::bail!(
                "task repo not set. Use --tasks-github-task-repo owner/repo or set task_repo in the config file.\n  \
                 This is the GitHub repository whose issues the dispatcher processes."
            );
        }
        if self.token.is_empty() {
            anyhow::bail!(
                "GitHub token not set. Set token in [tasks.github] config or use --tasks-github-task-github-token.\n  \
                 This token needs read/write access to the tasks repo."
            );
        }
        Ok(())
    }

    /// Parse "owner/repo" into (owner, repo).
    pub fn parse_repo(&self) -> anyhow::Result<(&str, &str)> {
        let parts: Vec<&str> = self.task_repo.splitn(2, '/').collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid task_repo format '{}', expected 'owner/repo'",
                self.task_repo
            );
        }
        Ok((parts[0], parts[1]))
    }
}

impl zbobr_api::config::BackendWithConfig for crate::ZbobrTaskBackendGithub {
    type Config = ZbobrTaskBackendGithubConfig;
}
