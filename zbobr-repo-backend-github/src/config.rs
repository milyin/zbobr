use std::path::PathBuf;

use zbobr_api::Secret;
use zbobr_utility::config_struct;

#[derive(Clone)]
#[config_struct]
/// Configuration for the GitHub repo backend.
pub struct ZbobrRepoBackendGithubConfig {
    /// Owner for forks (GitHub user or org).
    #[arg(long)]
    pub fork_owner: String,
    /// GitHub token with read/write access to fork org.
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
            fork_owner: String::new(),
            github_token: Secret::default(),
            repos_dir: PathBuf::from("./repos"),
        }
    }
}

impl ZbobrRepoBackendGithubConfig {
    /// Validate that all required fields are set and resolve secrets.
    pub fn validate(&mut self) -> anyhow::Result<()> {
        if self.fork_owner.is_empty() {
            anyhow::bail!(
                "fork owner not set. Use --repo-fork-owner NAME or set fork_owner in [repo] config.\n  \
                 This is the GitHub user or organization where target repos are forked for implementation."
            );
        }
        let token = self.github_token.resolve().map_err(|e| {
            anyhow::anyhow!(
                "GitHub token not set. Set github_token in [repo] config.\n  \
                 This token needs read/write access to the organization where repos are forked.\n  \
                 Error: {e}"
            )
        })?;
        if token.is_empty() {
            anyhow::bail!(
                "GitHub token not set. Set github_token in [repo] config.\n  \
                 This token needs read/write access to the organization where repos are forked."
            );
        }
        Ok(())
    }
}
