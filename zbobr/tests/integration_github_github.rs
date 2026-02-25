/// Integration tests: GitHub task backend + GitHub repo backend.
///
/// All tests in this file are marked `#[ignore]` and will not run unless
/// explicitly requested with `cargo test -- --ignored` or
/// `cargo test --test integration_github_github -- --ignored`.
///
/// Requires `zbobr_github_test.toml` to have both `[tasks.github]` and
/// `[repo.github]` sections with valid credentials.
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
        let repo = cfg.repo?;

        IntegrationTestEnv::init(
            "github_github",
            TaskBackendArgs::GitHub {
                task_repo: tasks.github.task_repo,
                task_token: tasks.github.token,
            },
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
// Tests (all ignored by default)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_preparation() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_preparation(&env).await;
}

#[tokio::test]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_planning() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_planning(&env).await;
}

#[tokio::test]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_working() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_working(&env).await;
}

#[tokio::test]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_reviewing() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_reviewing(&env).await;
}

#[tokio::test]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_merging() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_merging(&env).await;
}

#[tokio::test]
#[ignore = "full GitHub backend test — run with `cargo test -- --ignored`"]
async fn test_github_github_merging_with_real_conflict() {
    let Some(env) = get_env().await else { return };
    test_helpers::run_merging_with_real_conflict(&env).await;
}
