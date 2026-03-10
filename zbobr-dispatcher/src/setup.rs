use crate::ZbobrDispatcher;

impl ZbobrDispatcher {
    /// Set up the task repository: create if not exists, update stages and labels.
    pub async fn setup(&self, force: bool) -> anyhow::Result<()> {
        self.setup_repository(force).await
    }
}
