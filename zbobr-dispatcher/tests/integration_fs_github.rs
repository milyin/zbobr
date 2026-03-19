/// Integration tests: filesystem task backend + GitHub repo backend.
///
/// All tests are `#[ignore]` by default; run explicitly with:
///   cargo test --test integration_fs_github -- --ignored
///
/// Requires `zbobr_github_test.toml` at the workspace root with a
/// `[repo]` section.
mod mcp_integration;

use std::sync::Arc;

use mcp_integration::{IntegrationTestEnv, abstract_test_helpers, github_config::GitHubTestConfig};
use serial_test::serial;
use tokio::sync::OnceCell;

/// Credentials cached once from the config file.  A **fresh** `IntegrationTestEnv`
/// is created for every test so its octocrab/reqwest client is always bound to
/// the current tokio runtime, avoiding `Service { source: Closed }` errors that
/// occur when a pooled HTTP connection outlives the runtime that created it.
static CONFIG: OnceCell<(String, String, Option<String>)> = OnceCell::const_new();

async fn load_credentials() -> (String, String, Option<String>) {
    let creds: &(String, String, Option<String>) = CONFIG
        .get_or_init(|| async {
            let cfg = GitHubTestConfig::load()
                .expect("zbobr_github_test.toml not found; required for FS/GitHub tests");
            let repo = cfg
                .repo
                .expect("[repo] section missing in zbobr_github_test.toml");
            let target_repo = cfg.tasks.as_ref().and_then(|t| t.github_repo.clone());
            (
                repo.fork_owner.expect("fork_owner missing in [repo]"),
                repo.github_token.expect("github_token missing in [repo]"),
                target_repo,
            )
        })
        .await;
    creds.clone()
}

async fn get_env() -> Arc<IntegrationTestEnv> {
    let (fork_owner, repo_token, target_repo) = load_credentials().await;
    mcp_integration::env::init_fs_github("fs_github", fork_owner, repo_token, target_repo)
        .await
        .expect("failed to initialise FS/GitHub environment; check credentials")
}

// ---------------------------------------------------------------------------
// Abstract pipeline tests (generic stage/mode names)
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_abstract_all_mcp_tools() {
    let env = get_env().await;
    abstract_test_helpers::run_all_mcp_tools(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_abstract_stage_transfer() {
    let env = get_env().await;
    abstract_test_helpers::run_stage_transfer(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_abstract_call_mode() {
    let env = get_env().await;
    abstract_test_helpers::run_call_mode(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_abstract_return_from_mode() {
    let env = get_env().await;
    abstract_test_helpers::run_return_from_mode(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_abstract_auto_conflict() {
    let env = get_env().await;
    abstract_test_helpers::run_auto_conflict(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_abstract_pause_on_error() {
    let env = get_env().await;
    abstract_test_helpers::run_pause_on_error(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_abstract_ready_dispatch() {
    let env = get_env().await;
    abstract_test_helpers::run_ready_dispatch(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_abstract_signal_transitions() {
    let env = get_env().await;
    abstract_test_helpers::run_signal_transitions(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_abstract_pause_on_ask_user() {
    let env = get_env().await;
    abstract_test_helpers::run_pause_on_ask_user(&env).await;
}
