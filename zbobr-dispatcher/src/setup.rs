use crate::{ZbobrDispatcher, backend::TaskBackend};

impl ZbobrDispatcher {
    /// Set up the task repository: create if not exists, update stages and labels.
    pub async fn setup(&self, task_backend: &dyn TaskBackend, force: bool) -> anyhow::Result<()> {
        self.setup_repository(task_backend, force).await
    }
}
