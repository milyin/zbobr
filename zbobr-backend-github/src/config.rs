use zbobr_dispatcher::ZbobrError;

/// TOML configuration for the GitHub backend.
/// All fields are optional — missing fields fall back to defaults.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZbobrBackendGithubToml {
    pub task_repo: Option<String>,
}

/// Resolved configuration for the GitHub backend.
#[derive(Debug, Clone, Default)]
pub struct ZbobrBackendGithubConfig {
    /// Task project repository ("Org/repo").
    pub task_repo: String,
}

impl ZbobrBackendGithubConfig {
    /// Build configuration by layering: defaults < TOML.
    pub fn build(toml: Option<&ZbobrBackendGithubToml>) -> Self {
        let defaults = Self::default();

        let task_repo = toml
            .and_then(|t| t.task_repo.clone())
            .unwrap_or(defaults.task_repo);

        Self { task_repo }
    }

    /// Validate that all required fields are set.
    pub fn validate(&self) -> Result<(), ZbobrError> {
        if self.task_repo.is_empty() {
            return Err(ZbobrError::Config(
                "task repo not set. Use --task-repo owner/repo or set task_repo in the config file.\n  \
                 This is the GitHub repository whose issues the dispatcher processes."
                    .into(),
            ));
        }
        Ok(())
    }

    /// Parse "owner/repo" into (owner, repo).
    pub fn parse_repo(&self) -> Result<(&str, &str), ZbobrError> {
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
