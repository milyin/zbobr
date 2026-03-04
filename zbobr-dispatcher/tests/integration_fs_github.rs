#![allow(clippy::await_holding_lock)]
/// Integration tests: filesystem task backend + GitHub repo backend.
///
/// All tests are `#[ignore]` by default; run explicitly with:
///   cargo test --test integration_fs_github -- --ignored
///
/// Requires `zbobr_github_test.toml` at the workspace root with a
/// `[repo.github]` section.
mod mcp_integration;

use std::sync::Arc;

use mcp_integration::{IntegrationTestEnv, github_config::GitHubTestConfig, test_helpers};
use tokio::sync::OnceCell;

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Credentials cached once from the config file.  A **fresh** `IntegrationTestEnv`
/// is created for every test so its octocrab/reqwest client is always bound to
/// the current tokio runtime, avoiding `Service { source: Closed }` errors that
/// occur when a pooled HTTP connection outlives the runtime that created it.
static CONFIG: OnceCell<(String, String, Option<String>)> = OnceCell::const_new();

async fn load_credentials() -> (String, String, Option<String>) {
    CONFIG
        .get_or_init(|| async {
            let cfg = GitHubTestConfig::load()
                .expect("zbobr_github_test.toml not found; required for FS/GitHub tests");
            let repo = cfg
                .repo
                .expect("[repo.github] section missing in zbobr_github_test.toml");
            let target_repo = cfg.tasks.as_ref().map(|t| t.github.github_repo.clone());
            (
                repo.github.fork_owner,
                repo.github.github_token,
                target_repo,
            )
        })
        .await
        .clone()
}

async fn get_env() -> Arc<IntegrationTestEnv> {
    let (fork_owner, repo_token, target_repo) = load_credentials().await;
    IntegrationTestEnv::init_fs_github("fs_github", fork_owner, repo_token, target_repo)
        .await
        .expect("failed to initialise FS/GitHub environment; check credentials")
}

// ---------------------------------------------------------------------------
// Core stage tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_preparation() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_preparation(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_planning() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_planning(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_working() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_working(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_reviewing() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_reviewing(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_merging() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_merging(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_merging_with_real_conflict() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_merging_with_real_conflict(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_conflict_detection() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_conflict_detection(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_reviewing_approval() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_reviewing_approval(&env).await;
}

// ---------------------------------------------------------------------------
// Repo backend tests — same-org
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn test_fs_github_repo_backend_clone() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_repo_backend_clone(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn test_fs_github_repo_backend_planning() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_repo_backend_planning(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn test_fs_github_repo_backend_working() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_repo_backend_working(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn test_fs_github_repo_backend_reviewing() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_repo_backend_reviewing(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn test_fs_github_repo_backend_merging() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_repo_backend_merging(&env).await;
}

// ---------------------------------------------------------------------------
// Repo backend tests — cross-org
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_repo_backend_clone_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_repo_backend_clone_cross_org(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_repo_backend_planning_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_repo_backend_planning_cross_org(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_repo_backend_working_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_repo_backend_working_cross_org(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_repo_backend_reviewing_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_repo_backend_reviewing_cross_org(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_repo_backend_merging_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_repo_backend_merging_cross_org(&env).await;
}

// ---------------------------------------------------------------------------
// Signal and confirm flag
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_report_error_preserves_signal() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_report_error_preserves_signal(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_cli_confirm_flag_pauses_on_stage_change() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let env = get_env().await;
    test_helpers::run_cli_confirm_flag(&env).await;
}
