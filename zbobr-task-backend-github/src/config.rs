/// TOML configuration for the GitHub task backend.
/// All fields are optional — missing fields fall back to defaults.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZbobrTaskBackendGithubToml {
    pub task_repo: Option<String>,
    pub github_token: Option<String>,
}

/// Resolved configuration for the GitHub task backend.
#[derive(Debug, Clone, Default)]
pub(crate) struct ZbobrTaskBackendGithubConfig {
    /// Task project repository ("Org/repo").
    pub(crate) task_repo: String,
    /// GitHub token with read/write access to tasks repo.
    pub(crate) github_token: String,
}

impl ZbobrTaskBackendGithubConfig {
    /// Build configuration by layering: defaults < env < TOML < overrides.
    pub(crate) fn build(
        toml: Option<&ZbobrTaskBackendGithubToml>,
        task_repo_override: Option<&str>,
    ) -> Self {
        let defaults = Self::default();

        let task_repo = task_repo_override
            .map(String::from)
            .or_else(|| toml.and_then(|t| t.task_repo.clone()))
            .unwrap_or(defaults.task_repo);

        // github_token: GH_TOKEN > GITHUB_TOKEN > TOML
        let github_token = std::env::var("GH_TOKEN")
            .ok()
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
            .or_else(|| toml.and_then(|t| t.github_token.clone()))
            .unwrap_or(defaults.github_token);

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
                "GitHub token not set. Set GH_TOKEN or GITHUB_TOKEN env var, or set github_token in [task.github] config.\n  \
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
