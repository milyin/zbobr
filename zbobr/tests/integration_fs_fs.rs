/// Integration tests: filesystem task backend + filesystem repo backend.
///
/// These tests are always active (no GitHub credentials required).
/// Run this group with: `cargo test --test integration_fs_fs`
/// or filter by prefix: `cargo test test_fs_fs_`
mod mcp_integration;

use std::sync::Arc;
use tokio::sync::OnceCell;

use mcp_integration::IntegrationTestEnv;
use mcp_integration::env::{RepoBackendArgs, TaskBackendArgs};
use mcp_integration::test_helpers;

static ENV: OnceCell<Option<Arc<IntegrationTestEnv>>> = OnceCell::const_new();
static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn get_env() -> Option<Arc<IntegrationTestEnv>> {
    ENV.get_or_init(|| async {
        let base = match std::env::var("CARGO_TARGET_TMPDIR") {
            Ok(p) => std::path::PathBuf::from(p).join("integration_env_fs_fs"),
            Err(_) => std::env::temp_dir().join("zbobr_integration_env_fs_fs"),
        };
        let tasks_dir = base.join("tasks");

        IntegrationTestEnv::init(
            "fs_fs",
            TaskBackendArgs::Filesystem { tasks_dir },
            RepoBackendArgs::Filesystem,
            "dummy-not-used".to_string(),
            None,
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
async fn test_fs_fs_preparation() {
    let _guard = TEST_LOCK.lock().await;
    let Some(env) = get_env().await else { return };
    test_helpers::run_preparation(&env).await;
}

#[tokio::test]
async fn test_fs_fs_planning() {
    let _guard = TEST_LOCK.lock().await;
    let Some(env) = get_env().await else { return };
    test_helpers::run_planning(&env).await;
}

#[tokio::test]
async fn test_fs_fs_working() {
    let _guard = TEST_LOCK.lock().await;
    let Some(env) = get_env().await else { return };
    test_helpers::run_working(&env).await;
}

#[tokio::test]
async fn test_fs_fs_reviewing() {
    let _guard = TEST_LOCK.lock().await;
    let Some(env) = get_env().await else { return };
    test_helpers::run_reviewing(&env).await;
}

#[tokio::test]
async fn test_fs_fs_merging() {
    let _guard = TEST_LOCK.lock().await;
    let Some(env) = get_env().await else { return };
    test_helpers::run_merging(&env).await;
}

#[tokio::test]
async fn test_fs_fs_merging_with_real_conflict() {
    let _guard = TEST_LOCK.lock().await;
    let Some(env) = get_env().await else { return };
    test_helpers::run_merging_with_real_conflict(&env).await;
}
