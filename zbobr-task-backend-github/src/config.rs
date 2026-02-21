use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the GitHub task backend.
pub struct ZbobrTaskBackendGithub {
    /// Task project repository ("Org/repo").
    #[arg(long)]
    pub task_repo: String,
    /// GitHub token with read/write access to tasks repo.
    #[arg(long, env = "ZBOBR_TASK_GITHUB_TOKEN")]
    pub token: String,
}

impl ZbobrTaskBackendGithubConfig {
    /// Build configuration by layering: defaults < TOML < args.
    pub(crate) fn build(
        toml: Option<ZbobrTaskBackendGithubToml>,
        args: ZbobrTaskBackendGithubArgs,
    ) -> Self {
        let defaults = Self::default();
        let merged = toml.unwrap_or_default().merge_with_args(args);

        let task_repo = merged.task_repo.unwrap_or(defaults.task_repo);
        let token = merged.token.unwrap_or(defaults.token);

        Self {
            task_repo,
            token,
        }
    }

    /// Validate that all required fields are set.
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
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
