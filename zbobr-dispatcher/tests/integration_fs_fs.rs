#![allow(clippy::await_holding_lock)]
/// Integration tests: filesystem task backend + filesystem repo backend.
///
/// These tests are always active (no GitHub credentials required).
/// Run this group with: `cargo test --test integration_fs_fs`
/// or filter by prefix: `cargo test test_fs_fs_`
mod mcp_integration;

use std::sync::Arc;

use mcp_integration::{IntegrationTestEnv, abstract_test_helpers};
use tokio::sync::OnceCell;

static ENV: OnceCell<Option<Arc<IntegrationTestEnv>>> = OnceCell::const_new();
// tokio::sync::Mutex serializes tests — no poison semantics, works across runtimes.
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn get_env() -> Option<Arc<IntegrationTestEnv>> {
    ENV.get_or_init(|| async { mcp_integration::env::init_fs_fs("fs_fs").await })
        .await
        .clone()
}

// ---------------------------------------------------------------------------
// Abstract pipeline tests (generic stage/mode names)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_fs_fs_abstract_all_mcp_tools() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_all_mcp_tools(&env).await;
}

#[tokio::test]
async fn test_fs_fs_abstract_stage_transfer() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_stage_transfer(&env).await;
}

#[tokio::test]
async fn test_fs_fs_abstract_call_mode() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_call_mode(&env).await;
}

#[tokio::test]
async fn test_fs_fs_abstract_return_from_mode() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_return_from_mode(&env).await;
}

#[tokio::test]
async fn test_fs_fs_abstract_auto_conflict() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_auto_conflict(&env).await;
}

#[tokio::test]
async fn test_fs_fs_abstract_pause_on_error() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_pause_on_error(&env).await;
}

#[tokio::test]
async fn test_fs_fs_abstract_ready_dispatch() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_ready_dispatch(&env).await;
}

#[tokio::test]
async fn test_fs_fs_abstract_signal_transitions() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_signal_transitions(&env).await;
}

#[tokio::test]
async fn test_fs_fs_abstract_pause_on_ask_user() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_pause_on_ask_user(&env).await;
}
