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
