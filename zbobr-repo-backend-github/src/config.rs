use std::path::PathBuf;

use zbobr_api::Secret;
use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Configuration for the GitHub repo backend.
pub struct ZbobrRepoBackendGithubConfig {
    /// GitHub repository to work in, in "owner/repo" format.
    #[arg(long)]
    pub repository: String,
    /// Base branch to work against (e.g. "main").
    #[arg(long)]
    pub branch: String,
    /// GitHub token with read/write access to the repository.
    /// Use `{ value = "token" }` for an inline token or `{ env = "VAR" }` to read from an env var.
    #[config(skip_args)]
    pub github_token: Secret,
    /// Directory for bare clones of repositories.
    #[arg(long)]
    #[config(path)]
    pub repos_dir: PathBuf,
}

impl Default for ZbobrRepoBackendGithubConfig {
    fn default() -> Self {
        Self {
            repository: String::new(),
            branch: "main".to_string(),
            github_token: Secret::default(),
            repos_dir: PathBuf::from("./repos"),
        }
    }
}

impl ZbobrRepoBackendGithubConfig {
    /// Validate that all required fields are set and resolve secrets.
    pub fn validate(&mut self) -> anyhow::Result<()> {
        if self.repository.is_empty() {
            anyhow::bail!(
                "repository not set. Set repository in [repo] config.\n  \
                 This is the GitHub repository in 'owner/repo' format."
            );
        }
        let token = self.github_token.resolve().map_err(|e| {
            anyhow::anyhow!(
                "GitHub token not set. Set github_token in [repo] config.\n  \
                 This token needs read/write access to the repository.\n  \
                 Error: {e}"
            )
        })?;
        if token.is_empty() {
            anyhow::bail!(
                "GitHub token not set. Set github_token in [repo] config.\n  \
                 This token needs read/write access to the repository."
            );
        }
        Ok(())
    }

    /// Extract the short repository name (the part after '/').
    pub fn repo_short_name(&self) -> &str {
        self.repository.rsplit('/').next().unwrap_or(&self.repository)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_repo(repository: &str) -> ZbobrRepoBackendGithubConfig {
        ZbobrRepoBackendGithubConfig {
            repository: repository.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn repo_short_name_owner_repo() {
        assert_eq!(config_with_repo("owner/my-repo").repo_short_name(), "my-repo");
    }

    #[test]
    fn repo_short_name_bare_name() {
        assert_eq!(config_with_repo("my-repo").repo_short_name(), "my-repo");
    }

    #[test]
    fn repo_short_name_nested_path() {
        assert_eq!(config_with_repo("org/sub/repo").repo_short_name(), "repo");
    }
}
