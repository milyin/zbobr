use crate::{Backends, ZbobrDispatcher};

impl ZbobrDispatcher {
    /// Set up the task repository: create if not exists, update stages and labels.
    pub async fn setup(&self, backends: &Backends, force: bool) -> anyhow::Result<()> {
        self.setup_repository(backends, force).await
    }
}
