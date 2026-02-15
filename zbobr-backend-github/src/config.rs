use zbobr_dispatcher::ZbobrError;

/// TOML configuration for the GitHub backend.
/// All fields are optional — missing fields fall back to defaults.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZbobrBackendGithubToml {
    pub task_repo: Option<String>,
    pub fork_owner: Option<String>,
}

/// Resolved configuration for the GitHub backend.
#[derive(Debug, Clone, Default)]
pub(crate) struct ZbobrBackendGithubConfig {
    /// Task project repository ("Org/repo").
    pub(crate) task_repo: String,
    /// Owner for forks (GitHub user or org).
    pub(crate) fork_owner: String,
}

impl ZbobrBackendGithubConfig {
    /// Build configuration by layering: defaults < TOML < overrides.
    pub(crate) fn build(
        toml: Option<&ZbobrBackendGithubToml>,
        task_repo_override: Option<&str>,
        fork_owner_override: Option<&str>,
    ) -> Self {
        let defaults = Self::default();

        let task_repo = task_repo_override
            .map(String::from)
            .or_else(|| toml.and_then(|t| t.task_repo.clone()))
            .unwrap_or(defaults.task_repo);

        let fork_owner = fork_owner_override
            .map(String::from)
            .or_else(|| toml.and_then(|t| t.fork_owner.clone()))
            .unwrap_or(defaults.fork_owner);

        Self { task_repo, fork_owner }
    }

    /// Validate that all required fields are set.
    pub(crate) fn validate(&self) -> Result<(), ZbobrError> {
        if self.task_repo.is_empty() {
            return Err(ZbobrError::Config(
                "task repo not set. Use --task-repo owner/repo or set task_repo in the config file.\n  \
                 This is the GitHub repository whose issues the dispatcher processes."
                    .into(),
            ));
        }
        if self.fork_owner.is_empty() {
            return Err(ZbobrError::Config(
                "fork owner not set. Use --fork-owner NAME or set fork_owner in [backend_github] config.\n  \
                 This is the GitHub user or organization where target repos are forked for implementation."
                    .into(),
            ));
        }
        Ok(())
    }

    /// Parse "owner/repo" into (owner, repo).
    pub(crate) fn parse_repo(&self) -> Result<(&str, &str), ZbobrError> {
        let parts: Vec<&str> = self.task_repo.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(ZbobrError::Config(format!(
                "Invalid task_repo format '{}', expected 'owner/repo'",
                self.task_repo
            )));
        }
        Ok((parts[0], parts[1]))
    }
}
