use std::path::PathBuf;

use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Configuration for the filesystem repo backend.
pub struct ZbobrRepoBackendFsConfig {
    /// Local path or remote URL of the repository to work in.
    #[arg(long)]
    pub repository: String,
    /// Base branch to work against (e.g. "main").
    #[arg(long)]
    pub branch: String,
    #[arg(long)]
    #[config(path)]
    pub repos_dir: PathBuf,
}

impl Default for ZbobrRepoBackendFsConfig {
    fn default() -> Self {
        Self {
            repository: String::new(),
            branch: "main".to_string(),
            repos_dir: PathBuf::from("./repos"),
        }
    }
}

impl ZbobrRepoBackendFsConfig {
    /// Validate that all required fields are set.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.repository.is_empty() {
            anyhow::bail!(
                "repository not set. Set repository in [repo] config.\n  \
                 This is the local path or remote URL of the repository to work in."
            );
        }
        if self.branch.is_empty() {
            anyhow::bail!(
                "branch not set. Set branch in [repo] config.\n  \
                 This is the base branch to work against (e.g. 'main')."
            );
        }
        // repos_dir can be any path — we'll create it if needed
        Ok(())
    }

    /// Extract the short name from the configured repository path/URL.
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

    fn config_with_repo(repository: &str) -> ZbobrRepoBackendFsConfig {
        ZbobrRepoBackendFsConfig {
            repository: repository.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn repo_short_name_simple_path() {
        assert_eq!(config_with_repo("/home/user/my-project").repo_short_name(), "my-project");
    }

    #[test]
    fn repo_short_name_trailing_slash() {
        assert_eq!(config_with_repo("/home/user/my-project/").repo_short_name(), "my-project");
    }

    #[test]
    fn repo_short_name_git_suffix() {
        assert_eq!(config_with_repo("/home/user/my-project.git").repo_short_name(), "my-project");
    }

    #[test]
    fn repo_short_name_git_url() {
        assert_eq!(
            config_with_repo("https://github.com/owner/repo.git").repo_short_name(),
            "repo"
        );
    }

    #[test]
    fn repo_short_name_trailing_slash_and_git() {
        assert_eq!(
            config_with_repo("https://github.com/owner/repo.git/").repo_short_name(),
            "repo"
        );
    }

    #[test]
    fn repo_short_name_bare_name() {
        assert_eq!(config_with_repo("my-repo").repo_short_name(), "my-repo");
    }

    #[test]
    fn validate_ok_when_repository_and_branch_set() {
        let config = ZbobrRepoBackendFsConfig {
            repository: "/home/user/repo".to_string(),
            branch: "main".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_fails_when_repository_empty() {
        let config = ZbobrRepoBackendFsConfig {
            repository: String::new(),
            branch: "main".to_string(),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("repository not set"));
    }

    #[test]
    fn validate_fails_when_branch_empty() {
        let config = ZbobrRepoBackendFsConfig {
            repository: "/home/user/repo".to_string(),
            branch: String::new(),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("branch not set"));
    }
}
