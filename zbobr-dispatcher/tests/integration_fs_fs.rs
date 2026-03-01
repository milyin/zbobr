#![allow(clippy::await_holding_lock)]
/// Integration tests: filesystem task backend + filesystem repo backend.
///
/// These tests are always active (no GitHub credentials required).
/// Run this group with: `cargo test --test integration_fs_fs`
/// or filter by prefix: `cargo test test_fs_fs_`
mod mcp_integration;


use std::sync::Arc;
use tokio::sync::OnceCell;

use mcp_integration::IntegrationTestEnv;
use mcp_integration::test_helpers;

static ENV: OnceCell<Option<Arc<IntegrationTestEnv>>> = OnceCell::const_new();
// tokio::sync::Mutex serializes tests — no poison semantics, works across runtimes.
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn get_env() -> Option<Arc<IntegrationTestEnv>> {
    ENV.get_or_init(|| async { IntegrationTestEnv::init_fs_fs("fs_fs").await })
        .await
        .clone()
}

// ---------------------------------------------------------------------------
// Core stage tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_fs_fs_preparation() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_preparation(&env).await;
}

#[tokio::test]
async fn test_fs_fs_planning() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_planning(&env).await;
}

#[tokio::test]
async fn test_fs_fs_working() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_working(&env).await;
}

#[tokio::test]
async fn test_fs_fs_reviewing() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_reviewing(&env).await;
}

#[tokio::test]
async fn test_fs_fs_merging() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_merging(&env).await;
}

#[tokio::test]
async fn test_fs_fs_merging_with_real_conflict() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_merging_with_real_conflict(&env).await;
}

#[tokio::test]
async fn test_fs_fs_conflict_detection() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_conflict_detection(&env).await;
}

#[tokio::test]
async fn test_fs_fs_report_error_preserves_signal() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_report_error_preserves_signal(&env).await;
}

#[tokio::test]
async fn test_fs_fs_signal_preservation_during_conflict() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_signal_preservation_during_conflict(&env).await;
}

#[tokio::test]
async fn test_fs_fs_reviewing_approval() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_reviewing_approval(&env).await;
}

// ---------------------------------------------------------------------------
// Repo backend clone / full-pipeline tests (FS repo backend)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_fs_fs_repo_backend_clone() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_repo_backend_clone(&env).await;
}

#[tokio::test]
async fn test_fs_fs_repo_backend_planning() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_repo_backend_planning(&env).await;
}

#[tokio::test]
async fn test_fs_fs_repo_backend_working() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_repo_backend_working(&env).await;
}

#[tokio::test]
async fn test_fs_fs_repo_backend_reviewing() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_repo_backend_reviewing(&env).await;
}

#[tokio::test]
async fn test_fs_fs_repo_backend_merging() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_repo_backend_merging(&env).await;
}

// ---------------------------------------------------------------------------
// Cross-org tests (skipped for FS backend — fork_owner() returns None)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_fs_fs_repo_backend_clone_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_repo_backend_clone_cross_org(&env).await;
}

#[tokio::test]
async fn test_fs_fs_repo_backend_planning_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_repo_backend_planning_cross_org(&env).await;
}

#[tokio::test]
async fn test_fs_fs_repo_backend_working_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_repo_backend_working_cross_org(&env).await;
}

#[tokio::test]
async fn test_fs_fs_repo_backend_reviewing_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_repo_backend_reviewing_cross_org(&env).await;
}

#[tokio::test]
async fn test_fs_fs_repo_backend_merging_cross_org() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_repo_backend_merging_cross_org(&env).await;
}

// ---------------------------------------------------------------------------
// Confirm flag behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_fs_fs_cli_confirm_flag_pauses_on_stage_change() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    test_helpers::run_cli_confirm_flag(&env).await;
}
