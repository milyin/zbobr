use std::path::PathBuf;

/// Configuration loaded from `zbobr_github_test.toml` at the workspace root.
/// When this file is present the integration tests also run against the GitHub
/// backend.  Mirrors the relevant TOML sections of the standard zbobr config.
#[derive(serde::Deserialize, Clone)]
pub struct GitHubTestConfig {
    pub tasks: GitHubTasksSection,
    pub repo: GitHubRepoSection,
    #[serde(default)]
    pub dispatcher: GitHubDispatcherSection,
}

#[derive(serde::Deserialize, Clone)]
pub struct GitHubTasksSection {
    pub github: GitHubTasksGithub,
}

#[derive(serde::Deserialize, Clone)]
pub struct GitHubTasksGithub {
    /// GitHub repository used as the task tracker in `owner/repo` format.
    pub task_repo: String,
    /// GitHub token with read/write access to the tasks repository.
    pub token: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct GitHubRepoSection {
    pub github: GitHubRepoGithub,
}

#[derive(serde::Deserialize, Clone)]
pub struct GitHubRepoGithub {
    /// GitHub user or organisation where target repos are forked.
    pub fork_owner: String,
    /// GitHub token with read/write access to the fork organisation.
    pub token: String,
}

#[derive(serde::Deserialize, Clone, Default)]
pub struct GitHubDispatcherSection {
    /// GitHub token passed as `GH_TOKEN` to agent processes.
    /// Defaults to a dummy value, which is fine when `mcp-tester` is the executor.
    #[serde(default = "GitHubDispatcherSection::default_agent_token")]
    pub agent_token: String,
}

impl GitHubDispatcherSection {
    fn default_agent_token() -> String {
        "dummy-not-used".to_string()
    }
}

impl GitHubTestConfig {
    /// Try to load `zbobr_github_test.toml` from the workspace root.
    ///
    /// Returns `None` if the file does not exist.  Panics on parse errors so
    /// that a misconfigured file is caught early.
    pub fn load() -> Option<Self> {
        // `CARGO_MANIFEST_DIR` is the `zbobr/` package directory; its parent
        // is the workspace root where `zbobr_github_test.toml` lives.
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .expect("CARGO_MANIFEST_DIR must have a parent directory");
        let config_path = workspace_root.join("zbobr_github_test.toml");

        if !config_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&config_path)
            .expect("failed to read zbobr_github_test.toml");
        let config: GitHubTestConfig =
            toml::from_str(&content).expect("failed to parse zbobr_github_test.toml");
        Some(config)
    }
}
