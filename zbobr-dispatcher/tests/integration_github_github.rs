/// Integration tests: GitHub task backend + GitHub repo backend.
///
/// All tests are `#[ignore]` by default; run explicitly with:
///   cargo test --test integration_github_github -- --ignored
///
/// Requires `zbobr_github_test.toml` at the workspace root with both
/// `[tasks]` and `[repo]` sections.
mod mcp_integration;

use std::sync::Arc;

use mcp_integration::{IntegrationTestEnv, abstract_test_helpers, github_config::GitHubTestConfig};
use serial_test::serial;
use tokio::sync::OnceCell;

/// Credentials cached once from the config file.  A **fresh** `IntegrationTestEnv`
/// is created for every test so its octocrab/reqwest client is always bound to
/// the current tokio runtime, avoiding `Service { source: Closed }` errors that
/// occur when a pooled HTTP connection outlives the runtime that created it.
static CONFIG: OnceCell<(String, String, String, String, String)> = OnceCell::const_new();

async fn load_credentials() -> (String, String, String, String, String) {
    let creds: &(String, String, String, String, String) = CONFIG
        .get_or_init(|| async {
            let cfg = GitHubTestConfig::load()
                .expect("zbobr_github_test.toml not found; required for GitHub/GitHub tests");
            let tasks = cfg
                .tasks
                .expect("[tasks] section missing in zbobr_github_test.toml");
            let repo = cfg
                .repo
                .expect("[repo] section missing in zbobr_github_test.toml");
            let task_token = {
                let mut s = tasks.github_token.expect("github_token missing in [tasks]");
                s.resolve()
                    .expect("failed to resolve tasks.github_token")
                    .to_owned()
            };
            let repo_token = {
                let mut s = repo.github_token.expect("github_token missing in [repo]");
                s.resolve()
                    .expect("failed to resolve repo.github_token")
                    .to_owned()
            };
            (
                tasks.github_repo.expect("github_repo missing in [tasks]"),
                task_token,
                repo.repository.expect("repository missing in [repo]"),
                repo.branch.unwrap_or_else(|| "main".to_string()),
                repo_token,
            )
        })
        .await;
    creds.clone()
}

async fn get_env() -> Arc<IntegrationTestEnv> {
    let (task_repo, task_token, repository, branch, repo_token) = load_credentials().await;
    mcp_integration::env::init_github_github(
        "github_github",
        task_repo,
        task_token,
        repository,
        branch,
        repo_token,
    )
    .await
    .expect("failed to initialise GitHub/GitHub environment; check credentials")
}

// ---------------------------------------------------------------------------
// Abstract pipeline tests (generic stage/mode names)
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_abstract_all_mcp_tools() {
    let env = get_env().await;
    abstract_test_helpers::run_all_mcp_tools(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_abstract_stage_transfer() {
    let env = get_env().await;
    abstract_test_helpers::run_stage_transfer(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_abstract_auto_conflict() {
    let env = get_env().await;
    abstract_test_helpers::run_auto_conflict(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_abstract_pause_on_error() {
    let env = get_env().await;
    abstract_test_helpers::run_pause_on_error(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_abstract_ready_dispatch() {
    let env = get_env().await;
    abstract_test_helpers::run_ready_dispatch(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_abstract_signal_transitions() {
    let env = get_env().await;
    abstract_test_helpers::run_signal_transitions(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_abstract_pause_on_ask_user() {
    let env = get_env().await;
    abstract_test_helpers::run_pause_on_ask_user(&env).await;
}
