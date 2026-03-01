/*
use std::sync::Arc;

use tempfile::TempDir;
use zbobr_api::backend::TaskBackend;
use zbobr_task_backend_fs::ZbobrTaskBackendFs;

/// Test harness that holds a temporary directory and a configured fs backend.
pub struct TestSetup {
    /// Keep the TempDir alive so the directory isn't deleted while tests run.
    pub _tmp: TempDir,
    pub backend: Arc<dyn TaskBackend>,
}

/// Create a fresh filesystem backend pointed at a temporary directory.
pub fn create_test_setup() -> TestSetup {
    let tmp = TempDir::new().expect("failed to create temp dir");
    let tasks_dir = tmp.path().join("tasks");

    let backend = ZbobrTaskBackendFs::new(
        None,
        zbobr_task_backend_fs::ZbobrTaskBackendFsArgs {
            tasks_dir: Some(tasks_dir.to_path_buf()),
        },
        tmp.path(),
    )
    .expect("failed to create fs backend");

    TestSetup {
        _tmp: tmp,
        backend: Arc::new(backend),
    }
}

*/
