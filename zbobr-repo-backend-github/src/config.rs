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
    /// Automatically sync a forked repository with its upstream before each fetch.
    #[arg(long)]
    pub auto_sync_fork: bool,
}

impl Default for ZbobrRepoBackendGithubConfig {
    fn default() -> Self {
        Self {
            repository: String::new(),
            branch: "main".to_string(),
            github_token: Secret::default(),
            repos_dir: PathBuf::from("./repos"),
            auto_sync_fork: true,
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
        if self.branch.is_empty() {
            anyhow::bail!(
                "branch not set. Set branch in [repo] config.\n  \
                 This is the base branch to work against (e.g. 'main')."
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

    /// Extract the short repository name (the part after the last '/'),
    /// stripping any trailing slash or `.git` suffix first.
    pub fn repo_short_name(&self) -> &str {
        self.repository
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or(&self.repository)
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
        assert_eq!(
            config_with_repo("owner/my-repo").repo_short_name(),
            "my-repo"
        );
    }

    #[test]
    fn repo_short_name_bare_name() {
        assert_eq!(config_with_repo("my-repo").repo_short_name(), "my-repo");
    }

    #[test]
    fn repo_short_name_nested_path() {
        assert_eq!(config_with_repo("org/sub/repo").repo_short_name(), "repo");
    }

    #[test]
    fn repo_short_name_git_suffix() {
        assert_eq!(
            config_with_repo("owner/my-repo.git").repo_short_name(),
            "my-repo"
        );
    }

    #[test]
    fn repo_short_name_https_url() {
        assert_eq!(
            config_with_repo("https://github.com/owner/repo.git").repo_short_name(),
            "repo"
        );
    }

    #[test]
    fn repo_short_name_trailing_slash() {
        assert_eq!(
            config_with_repo("owner/my-repo/").repo_short_name(),
            "my-repo"
        );
    }

    fn config_with_repo_and_branch(
        repository: &str,
        branch: &str,
        token: Secret,
    ) -> ZbobrRepoBackendGithubConfig {
        ZbobrRepoBackendGithubConfig {
            repository: repository.to_string(),
            branch: branch.to_string(),
            github_token: token,
            ..Default::default()
        }
    }

    #[test]
    fn validate_ok_when_all_fields_set() {
        let mut config =
            config_with_repo_and_branch("owner/repo", "main", Secret::value("ghp_token123"));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_fails_when_repository_empty() {
        let mut config = config_with_repo_and_branch("", "main", Secret::value("ghp_token123"));
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("repository not set"));
    }

    #[test]
    fn validate_fails_when_branch_empty() {
        let mut config =
            config_with_repo_and_branch("owner/repo", "", Secret::value("ghp_token123"));
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("branch not set"));
    }

    #[test]
    fn validate_fails_when_token_empty() {
        let mut config = config_with_repo_and_branch("owner/repo", "main", Secret::value(""));
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("GitHub token not set"));
    }

    #[test]
    fn validate_fails_when_token_env_missing() {
        unsafe {
            std::env::remove_var("ZBOBR_TEST_VALIDATE_MISSING_TOKEN");
        }
        let mut config = config_with_repo_and_branch(
            "owner/repo",
            "main",
            Secret::env("ZBOBR_TEST_VALIDATE_MISSING_TOKEN"),
        );
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("GitHub token not set"));
    }
}
