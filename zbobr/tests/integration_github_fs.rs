/// Integration tests: GitHub task backend + filesystem repo backend.
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

static ENV: OnceCell<Option<Arc<IntegrationTestEnv>>> = OnceCell::const_new();

async fn get_env() -> Option<Arc<IntegrationTestEnv>> {
    ENV.get_or_init(|| async {
        let cfg = GitHubTestConfig::load()?;
        let tasks = cfg.tasks?;

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
    })
    .await
    .clone()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_github_fs_preparation() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_preparation(&env).await;
}

#[tokio::test]
async fn test_github_fs_planning() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_planning(&env).await;
}

#[tokio::test]
async fn test_github_fs_working() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_working(&env).await;
}

#[tokio::test]
async fn test_github_fs_reviewing() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_reviewing(&env).await;
}

#[tokio::test]
async fn test_github_fs_merging() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_merging(&env).await;
}

#[tokio::test]
async fn test_github_fs_merging_with_real_conflict() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_merging_with_real_conflict(&env).await;
}
