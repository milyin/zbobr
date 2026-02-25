/// Integration tests: filesystem task backend + GitHub repo backend.
///
/// Activated when `zbobr_github_test.toml` at the workspace root contains a
/// `[repo.github]` section with valid credentials.
/// Run this group with: `cargo test --test integration_fs_github`
/// or filter by prefix: `cargo test test_fs_github_`
mod mcp_integration;

use std::sync::Arc;
use tokio::sync::OnceCell;

use mcp_integration::env::{IntegrationTestEnv, RepoBackendArgs, TaskBackendArgs};
use mcp_integration::github_config::GitHubTestConfig;
use mcp_integration::test_helpers;

static ENV: OnceCell<Option<Arc<IntegrationTestEnv>>> = OnceCell::const_new();

async fn get_env() -> Option<Arc<IntegrationTestEnv>> {
    ENV.get_or_init(|| async {
        let cfg = GitHubTestConfig::load()?;
        let repo = cfg.repo?;

        let base = match std::env::var("CARGO_TARGET_TMPDIR") {
            Ok(p) => std::path::PathBuf::from(p).join("integration_env_fs_github"),
            Err(_) => std::env::temp_dir().join("zbobr_integration_env_fs_github"),
        };
        let tasks_dir = base.join("tasks");

        IntegrationTestEnv::init(
            "fs_github",
            TaskBackendArgs::Filesystem { tasks_dir },
            RepoBackendArgs::GitHub {
                fork_owner: repo.github.fork_owner,
                repo_token: repo.github.token,
            },
            cfg.dispatcher.agent_token,
        )
        .await
    })
    .await
    .clone()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_fs_github_preparation() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_preparation(&env).await;
}

#[tokio::test]
async fn test_fs_github_planning() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_planning(&env).await;
}

#[tokio::test]
async fn test_fs_github_working() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_working(&env).await;
}

#[tokio::test]
async fn test_fs_github_reviewing() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_reviewing(&env).await;
}

#[tokio::test]
async fn test_fs_github_merging() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_merging(&env).await;
}

#[tokio::test]
async fn test_fs_github_merging_with_real_conflict() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_merging_with_real_conflict(&env).await;
}
