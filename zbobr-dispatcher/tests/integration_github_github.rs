// This test is temporarily disabled while GitHub repo backend is being refactored.
// The GitHub backend is currently excluded from the build.
// Use integration_fs_fs.rs for FS-only backend tests.
//
// /// Integration tests: GitHub task backend + GitHub repo backend.
// ///
// /// All tests are `#[ignore]` by default; run explicitly with:
// ///   cargo test --test integration_github_github -- --ignored
// ///
// /// Requires `zbobr_github_test.toml` at the workspace root with both
// /// `[tasks]` and `[repo]` sections.
mod mcp_integration;

use std::sync::Arc;

use mcp_integration::{IntegrationTestEnv, github_config::GitHubTestConfig, test_helpers};
use serial_test::serial;
use tokio::sync::OnceCell;

/// Credentials cached once from the config file.  A **fresh** `IntegrationTestEnv`
/// is created for every test so its octocrab/reqwest client is always bound to
/// the current tokio runtime, avoiding `Service { source: Closed }` errors that
/// occur when a pooled HTTP connection outlives the runtime that created it.
static CONFIG: OnceCell<(String, String, String, String)> = OnceCell::const_new();

async fn load_credentials() -> (String, String, String, String) {
    let creds: &(String, String, String, String) = CONFIG
        .get_or_init(|| async {
            let cfg = GitHubTestConfig::load()
                .expect("zbobr_github_test.toml not found; required for GitHub/GitHub tests");
            let tasks = cfg
                .tasks
                .expect("[tasks] section missing in zbobr_github_test.toml");
            let repo = cfg
                .repo
                .expect("[repo] section missing in zbobr_github_test.toml");
            (
                tasks.github_repo.expect("github_repo missing in [tasks]"),
                tasks.github_token.expect("github_token missing in [tasks]"),
                repo.fork_owner.expect("fork_owner missing in [repo]"),
                repo.github_token.expect("github_token missing in [repo]"),
            )
        })
        .await;
    creds.clone()
}

async fn get_env() -> Arc<IntegrationTestEnv> {
    let (task_repo, task_token, fork_owner, repo_token) = load_credentials().await;
    IntegrationTestEnv::init_github_github(
        "github_github",
        task_repo,
        task_token,
        fork_owner,
        repo_token,
    )
    .await
    .expect("failed to initialise GitHub/GitHub environment; check credentials")
}

// ---------------------------------------------------------------------------
// Core stage tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_preparation() {
    let env = get_env().await;
    test_helpers::run_preparation(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_planning() {
    let env = get_env().await;
    test_helpers::run_planning(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_working() {
    let env = get_env().await;
    test_helpers::run_working(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_reviewing() {
    let env = get_env().await;
    test_helpers::run_reviewing(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_merging() {
    let env = get_env().await;
    test_helpers::run_merging(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_merging_with_real_conflict() {
    let env = get_env().await;
    test_helpers::run_merging_with_real_conflict(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_conflict_detection() {
    let env = get_env().await;
    test_helpers::run_conflict_detection(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_reviewing_approval() {
    let env = get_env().await;
    test_helpers::run_reviewing_approval(&env).await;
}

// ---------------------------------------------------------------------------
// Repo backend tests — same-org
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_repo_backend_clone() {
    let env = get_env().await;
    test_helpers::run_repo_backend_clone(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_repo_backend_planning() {
    let env = get_env().await;
    test_helpers::run_repo_backend_planning(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_repo_backend_working() {
    let env = get_env().await;
    test_helpers::run_repo_backend_working(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_repo_backend_reviewing() {
    let env = get_env().await;
    test_helpers::run_repo_backend_reviewing(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_repo_backend_merging() {
    let env = get_env().await;
    test_helpers::run_repo_backend_merging(&env).await;
}

// ---------------------------------------------------------------------------
// Repo backend tests — cross-org
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_repo_backend_clone_cross_org() {
    let env = get_env().await;
    test_helpers::run_repo_backend_clone_cross_org(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_repo_backend_planning_cross_org() {
    let env = get_env().await;
    test_helpers::run_repo_backend_planning_cross_org(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_repo_backend_working_cross_org() {
    let env = get_env().await;
    test_helpers::run_repo_backend_working_cross_org(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_repo_backend_reviewing_cross_org() {
    let env = get_env().await;
    test_helpers::run_repo_backend_reviewing_cross_org(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_repo_backend_merging_cross_org() {
    let env = get_env().await;
    test_helpers::run_repo_backend_merging_cross_org(&env).await;
}

// ---------------------------------------------------------------------------
// Signal and confirm flag
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_report_error_preserves_signal() {
    let env = get_env().await;
    test_helpers::run_report_error_preserves_signal(&env).await;
}

#[tokio::test]
#[serial]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_cli_confirm_flag_pauses_on_stage_change() {
    let env = get_env().await;
    test_helpers::run_cli_confirm_flag(&env).await;
}
