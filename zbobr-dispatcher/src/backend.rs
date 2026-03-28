// Re-export traits from zbobr-api so existing code still compiles.
pub use zbobr_api::backend::{TaskBackend, TaskBackendExt, TaskMut, TaskWeak, WorktreeBackend};
// Re-export helpers from zbobr-utility for backwards compatibility.
pub use zbobr_utility::{configure_git_user, create_placeholder_commit, delete_placeholder_commit};

/// A dummy backend used exclusively for extracting MCP tool schemas
/// without needing a real backend instance.
#[derive(Clone)]
pub struct DummyBackend;

#[async_trait::async_trait]
impl zbobr_api::backend::TaskBackend for DummyBackend {
    async fn get_task(&self, _id: u64) -> anyhow::Result<Box<dyn zbobr_api::backend::TaskWeak>> {
        unimplemented!()
    }
    async fn list_tasks(&self, _allowed_users: &[String]) -> anyhow::Result<Vec<Box<dyn zbobr_api::backend::TaskWeak>>> {
        unimplemented!()
    }
    async fn create_task(
        &self,
        _title: &str,
        _description: &str,
        _state: zbobr_api::State,
    ) -> anyhow::Result<u64> {
        unimplemented!()
    }
    async fn setup(&self, _force: bool) -> anyhow::Result<()> {
        unimplemented!()
    }
    async fn validate_connectivity(&self) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn debug_state(&self) -> String {
        unimplemented!()
    }
}
