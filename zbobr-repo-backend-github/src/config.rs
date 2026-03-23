use std::path::PathBuf;

use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Configuration for the GitHub repo backend.
pub struct ZbobrRepoBackendGithubConfig {
    /// Owner for forks (GitHub user or org).
    #[arg(long)]
    pub fork_owner: String,
    /// GitHub token with read/write access to fork org.
    #[arg(long, env = "ZBOBR_REPO_GITHUB_TOKEN")]
    pub github_token: String,
    /// Directory for bare clones of repositories.
    #[arg(long)]
    #[config(path)]
    pub repos_dir: PathBuf,
}

impl Default for ZbobrRepoBackendGithubConfig {
    fn default() -> Self {
        Self {
            fork_owner: String::new(),
            github_token: String::new(),
            repos_dir: PathBuf::from("./repos"),
        }
    }
}

impl ZbobrRepoBackendGithubConfig {
    /// Validate that all required fields are set.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.fork_owner.is_empty() {
            anyhow::bail!(
                "fork owner not set. Use --repo-fork-owner NAME or set fork_owner in [repo] config.\n  \
                 This is the GitHub user or organization where target repos are forked for implementation."
            );
        }
        if self.github_token.is_empty() {
            anyhow::bail!(
                "GitHub token not set. Set github_token in [repo] config or use --repo-github-token.\n  \
                 This token needs read/write access to the organization where repos are forked."
            );
        }
        Ok(())
    }
}
