#![allow(dead_code)]

use std::path::PathBuf;

use zbobr_dispatcher::GenericConfigToml;
use zbobr_repo_backend_github::ZbobrRepoBackendGithubConfig;
use zbobr_task_backend_github::ZbobrTaskBackendGithubConfig;

/// Configuration loaded from `zbobr_github_test.toml` at the workspace root.
///
/// Uses the production `GenericConfigToml` type with concrete GitHub backend
/// configs.  Each section is optional so that only the tests relevant to the
/// present credentials are activated:
/// - `[tasks]` → enables GitHub task-backend tests
/// - `[repo]`  → enables GitHub repo-backend tests
///
/// A section being absent causes the corresponding test combination to be
/// skipped gracefully rather than failing.
pub type GitHubTestConfigToml =
    GenericConfigToml<ZbobrTaskBackendGithubConfig, ZbobrRepoBackendGithubConfig>;

/// Wrapper around the parsed TOML that also carries a dispatcher section for
/// legacy callers that inspect `dispatcher.agent_token`.
pub struct GitHubTestConfig {
    pub tasks: Option<zbobr_task_backend_github::ZbobrTaskBackendGithubToml>,
    pub repo: Option<zbobr_repo_backend_github::ZbobrRepoBackendGithubToml>,
}

impl GitHubTestConfig {
    /// Try to load `zbobr_github_test.toml` from the workspace root.
    ///
    /// Returns `None` if the file does not exist. Panics on parse errors so
    /// that a misconfigured file is caught early.
    pub fn load() -> Option<Self> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .expect("CARGO_MANIFEST_DIR must have a parent directory");
        let config_path = workspace_root.join("zbobr_github_test.toml");

        if !config_path.exists() {
            return None;
        }

        let content =
            std::fs::read_to_string(&config_path).expect("failed to read zbobr_github_test.toml");
        let config: GitHubTestConfigToml =
            toml::from_str(&content).expect("failed to parse zbobr_github_test.toml");
        Some(Self {
            tasks: config.tasks,
            repo: config.repo,
        })
    }
}
