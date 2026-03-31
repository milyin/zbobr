use zbobr_api::Secret;
use zbobr_utility::config_struct;

#[derive(Clone, Default)]
#[config_struct]
/// Configuration for the GitHub task backend.
pub struct ZbobrTaskBackendGithubConfig {
    /// Instance name for label-based task filtering (`zbobr:<instance>`).
    /// Injected from ZbobrDispatcherConfig; any TOML value is overwritten at runtime.
    #[config(skip_args)]
    pub instance: String,
    /// Task project repository ("Org/repo").
    #[arg(long)]
    pub github_repo: String,
    /// GitHub token with read/write access to tasks repo.
    /// Use `{ value = "token" }` for an inline token or `{ env = "VAR" }` to read from an env var.
    #[config(skip_args)]
    pub github_token: Secret,
    /// Branch to store report files on (default: repo's default branch).
    /// Use this when the default branch has protection rules that prevent direct pushes.
    #[arg(long)]
    pub reports_branch: Option<String>,
    /// Path prefix for report files (default: "reports").
    #[arg(long)]
    pub reports_path: Option<String>,
    /// If specified, only process tasks created by these GitHub usernames.
    #[arg(long)]
    pub allowed_usernames: Option<Vec<String>>,
}

impl ZbobrTaskBackendGithubConfig {
    /// Validate that all required fields are set and resolve secrets.
    pub fn validate(&mut self) -> anyhow::Result<()> {
        if self.github_repo.is_empty() {
            anyhow::bail!(
                "task repo not set. Use --tasks-github-repo owner/repo or set github_repo in [tasks] config.\n  \
                 This is the GitHub repository whose issues the dispatcher processes."
            );
        }
        let token = self.github_token.resolve().map_err(|e| {
            anyhow::anyhow!(
                "GitHub token not set. Set github_token in [tasks] config.\n  \
                 This token needs read/write access to the tasks repo.\n  \
                 Error: {e}"
            )
        })?;
        if token.is_empty() {
            anyhow::bail!(
                "GitHub token not set. Set github_token in [tasks] config.\n  \
                 This token needs read/write access to the tasks repo."
            );
        }
        Ok(())
    }

    /// Parse "owner/repo" into (owner, repo).
    pub fn parse_repo(&self) -> anyhow::Result<(&str, &str)> {
        let parts: Vec<&str> = self.github_repo.splitn(2, '/').collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid github_repo format '{}', expected 'owner/repo'",
                self.github_repo
            );
        }
        Ok((parts[0], parts[1]))
    }
}
