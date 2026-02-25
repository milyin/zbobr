/// Integration tests: GitHub task backend + filesystem repo backend.
///
/// All of the individual tests are marked `#[ignore]` so they won’t run by
/// default; invoke explicitly once you’ve supplied credentials with
/// `cargo test --test integration_github_fs -- --ignored`.
///
/// Activated when `zbobr_github_test.toml` at the workspace root contains a
/// `[tasks.github]` section with valid credentials.
/// Run this group with: `cargo test --test integration_github_fs`
/// or filter by prefix: `cargo test test_github_fs_`
mod mcp_integration;

use std::sync::Arc;
use tokio::sync::OnceCell;

use mcp_integration::env::{IntegrationTestEnv, RepoBackendArgs, TaskBackendArgs};
use mcp_integration::github_config::GitHubTestConfig;
use mcp_integration::test_helpers;

// we no longer return an Option; missing config or setup errors should panic
static ENV: OnceCell<Arc<IntegrationTestEnv>> = OnceCell::const_new();
static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn get_env() -> Arc<IntegrationTestEnv> {
    // initialize the cell if necessary, panicking on missing configuration or
    // setup failures so that a test run with `--ignored` fails loudly instead of
    // silently skipping.
    ENV.get_or_init(|| async {
        let cfg = GitHubTestConfig::load()
            .expect("zbobr_github_test.toml not found; required for GitHub tests");
        let tasks = cfg.tasks
            .expect("[tasks.github] section missing in zbobr_github_test.toml");

        IntegrationTestEnv::init(
            "github_fs",
            TaskBackendArgs::GitHub {
                task_repo: tasks.github.task_repo,
                task_token: tasks.github.token,
            },
            RepoBackendArgs::Filesystem,
            cfg.dispatcher.agent_token,
        )
        .await
        .expect("failed to initialize GitHub/FS integration environment; check credentials")
    })
    .await
    .clone()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_preparation() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_preparation(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_planning() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_planning(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_working() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_working(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_reviewing() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_reviewing(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_merging() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_merging(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_merging_with_real_conflict() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_merging_with_real_conflict(&env).await;
}
