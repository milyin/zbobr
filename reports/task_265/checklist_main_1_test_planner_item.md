# Register `run_pause_on_runner_error` in GitHub integration tests

## What
Add a `test_github_github_abstract_pause_on_runner_error` test function in `zbobr-dispatcher/tests/integration_github_github.rs` that calls `abstract_test_helpers::run_pause_on_runner_error(&env).await`.

## Why
The abstract test helper already exists and is registered for the fs-fs backend, but not for the github-github backend. Both backends should exercise the same behavioral tests to ensure consistent behavior regardless of the storage layer.

## How
Follow the same pattern as `test_fs_fs_abstract_pause_on_runner_error` in `integration_fs_fs.rs`:

```rust
#[tokio::test]
async fn test_github_github_abstract_pause_on_runner_error() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_pause_on_runner_error(&env).await;
}
```

## Risk
Low — the abstract helper is already proven on fs-fs. This is purely wiring.