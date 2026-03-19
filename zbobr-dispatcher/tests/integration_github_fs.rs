#![allow(clippy::await_holding_lock)]
/// Integration tests: GitHub task backend + filesystem repo backend.
///
/// All tests are `#[ignore]` by default; run explicitly with:
///   cargo test --test integration_github_fs -- --ignored
///
/// Requires `zbobr_github_test.toml` at the workspace root with a
/// `[tasks]` section.
mod mcp_integration;

use std::sync::Arc;

use mcp_integration::{IntegrationTestEnv, abstract_test_helpers, github_config::GitHubTestConfig};
use tokio::sync::OnceCell;

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Credentials cached once from the config file.  A **fresh** `IntegrationTestEnv`
/// is created for every test so its octocrab/reqwest client is always bound to
/// the current tokio runtime, avoiding `Service { source: Closed }` errors that
/// occur when a pooled HTTP connection outlives the runtime that created it.
static CONFIG: OnceCell<(String, String)> = OnceCell::const_new();

async fn load_credentials() -> (String, String) {
    CONFIG
        .get_or_init(|| async {
            let cfg = GitHubTestConfig::load()
                .expect("zbobr_github_test.toml not found; required for GitHub/FS tests");
            let tasks = cfg
                .tasks
                .expect("[tasks] section missing in zbobr_github_test.toml");
            (
                tasks.github_repo.expect("github_repo missing in [tasks]"),
                tasks.github_token.expect("github_token missing in [tasks]"),
            )
        })
        .await
        .clone()
}

async fn get_env() -> Arc<IntegrationTestEnv> {
    let (github_repo, github_token) = load_credentials().await;
    mcp_integration::env::init_github_fs("github_fs", github_repo, github_token)
        .await
        .expect("failed to initialise GitHub/FS environment; check credentials")
}

// ---------------------------------------------------------------------------
// Abstract pipeline tests (generic stage/mode names)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_abstract_all_mcp_tools() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    abstract_test_helpers::run_all_mcp_tools(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_abstract_stage_transfer() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    abstract_test_helpers::run_stage_transfer(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_abstract_call_mode() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    abstract_test_helpers::run_call_mode(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_abstract_return_from_mode() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    abstract_test_helpers::run_return_from_mode(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_abstract_auto_conflict() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    abstract_test_helpers::run_auto_conflict(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_abstract_pause_on_error() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    abstract_test_helpers::run_pause_on_error(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_abstract_ready_dispatch() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    abstract_test_helpers::run_ready_dispatch(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_abstract_signal_transitions() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    abstract_test_helpers::run_signal_transitions(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_abstract_pause_on_ask_user() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    abstract_test_helpers::run_pause_on_ask_user(&env).await;
}
