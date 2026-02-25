#![allow(dead_code)]

use std::path::PathBuf;

/// Configuration loaded from `zbobr_github_test.toml` at the workspace root.
///
/// Each section is optional so that only the tests relevant to the present
/// credentials are activated:
/// - `tasks.github` → enables GitHub task-backend tests
/// - `repo.github`  → enables GitHub repo-backend tests
///
/// A section being absent causes the corresponding test combination to be
/// skipped gracefully rather than failing.
#[derive(serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct GitHubTestConfig {
    pub tasks: Option<GitHubTasksSection>,
    pub repo: Option<GitHubRepoSection>,
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

#[derive(serde::Deserialize, Clone)]
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

impl Default for GitHubDispatcherSection {
    fn default() -> Self {
        Self { agent_token: Self::default_agent_token() }
    }
}

impl GitHubTestConfig {
    /// Try to load `zbobr_github_test.toml` from the workspace root.
    ///
    /// Returns `None` if the file does not exist.  Panics on parse errors so
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

        let content = std::fs::read_to_string(&config_path)
            .expect("failed to read zbobr_github_test.toml");
        let config: GitHubTestConfig =
            toml::from_str(&content).expect("failed to parse zbobr_github_test.toml");
        Some(config)
    }
}
