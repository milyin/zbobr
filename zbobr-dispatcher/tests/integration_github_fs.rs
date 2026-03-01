/// Integration tests: GitHub task backend + filesystem repo backend.
///
/// All tests are `#[ignore]` by default; run explicitly with:
///   cargo test --test integration_github_fs -- --ignored
///
/// Requires `zbobr_github_test.toml` at the workspace root with a
/// `[tasks.github]` section.
mod mcp_integration;

use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

use mcp_integration::IntegrationTestEnv;
use mcp_integration::github_config::GitHubTestConfig;
use mcp_integration::test_helpers;

static ENV: OnceCell<Arc<IntegrationTestEnv>> = OnceCell::const_new();
static TEST_LOCK: Mutex<()> = Mutex::new(());

async fn get_env() -> Arc<IntegrationTestEnv> {
    ENV.get_or_init(|| async {
        let cfg = GitHubTestConfig::load()
            .expect("zbobr_github_test.toml not found; required for GitHub/FS tests");
        let tasks = cfg
            .tasks
            .expect("[tasks.github] section missing in zbobr_github_test.toml");

        IntegrationTestEnv::init_github_fs(
            "github_fs",
            tasks.github.github_repo,
            tasks.github.github_token,
        )
        .await
        .expect("failed to initialise GitHub/FS environment; check credentials")
    })
    .await
    .clone()
}

// ---------------------------------------------------------------------------
// Core stage tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_preparation() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_preparation(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_planning() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_planning(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_working() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_working(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_reviewing() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_reviewing(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_merging() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_merging(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_merging_with_real_conflict() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_merging_with_real_conflict(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_conflict_detection() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_conflict_detection(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_reviewing_approval() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_reviewing_approval(&env).await;
}

// ---------------------------------------------------------------------------
// Repo backend tests (FS backend — fork_owner() is None)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_repo_backend_clone() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_repo_backend_clone(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_repo_backend_planning() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_repo_backend_planning(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_repo_backend_working() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_repo_backend_working(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_repo_backend_reviewing() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_repo_backend_reviewing(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_repo_backend_merging() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_repo_backend_merging(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_repo_backend_clone_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_repo_backend_clone_cross_org(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_repo_backend_planning_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_repo_backend_planning_cross_org(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_repo_backend_working_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_repo_backend_working_cross_org(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_repo_backend_reviewing_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_repo_backend_reviewing_cross_org(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_repo_backend_merging_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_repo_backend_merging_cross_org(&env).await;
}

// ---------------------------------------------------------------------------
// Signal and confirm flag
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_report_error_preserves_signal() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_report_error_preserves_signal(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_github_fs_cli_confirm_flag_pauses_on_stage_change() {
    let _guard = TEST_LOCK.lock().unwrap();
    let env = get_env().await;
    test_helpers::run_cli_confirm_flag(&env).await;
}
