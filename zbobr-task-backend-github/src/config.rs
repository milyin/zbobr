use zbobr_utility::config_struct;

#[config_struct]
/// Configuration for the GitHub task backend.
pub struct ZbobrTaskBackendGithub {
    /// Task project repository ("Org/repo").
    #[arg(long)]
    pub task_repo: String,
    /// GitHub token with read/write access to tasks repo.
    #[arg(long = "task-github-token", env = "ZBOBR_TASK_GITHUB_TOKEN", id = "task_github_token")]
    pub github_token: String,
}

/// Resolved configuration for the GitHub task backend.
#[derive(Debug, Clone, Default)]
pub(crate) struct ZbobrTaskBackendGithubRuntimeConfig {
    /// Task project repository ("Org/repo").
    pub(crate) task_repo: String,
    /// GitHub token with read/write access to tasks repo.
    pub(crate) github_token: String,
}

impl ZbobrTaskBackendGithubRuntimeConfig {
    /// Build configuration by layering: defaults < TOML < args.
    pub(crate) fn build(
        toml: Option<ZbobrTaskBackendGithubToml>,
        args: ZbobrTaskBackendGithubArgs,
    ) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);

        let task_repo = merged.task_repo.unwrap_or(defaults.task_repo);
        let github_token = merged.github_token.unwrap_or(defaults.github_token);

        Self {
            task_repo,
            github_token,
        }
    }

    /// Validate that all required fields are set.
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        if self.task_repo.is_empty() {
            anyhow::bail!(
                "task repo not set. Use --task-repo owner/repo or set task_repo in the config file.\n  \
                 This is the GitHub repository whose issues the dispatcher processes."
            );
        }
        if self.github_token.is_empty() {
            anyhow::bail!(
                "GitHub token not set. Set github_token in [task.github] config or use --github-token.\n  \
                 This token needs read/write access to the tasks repo."
            );
        }
        Ok(())
    }

    /// Parse "owner/repo" into (owner, repo).
    pub(crate) fn parse_repo(&self) -> anyhow::Result<(&str, &str)> {
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
