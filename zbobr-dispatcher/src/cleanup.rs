use crate::{Backends, TaskDir, ZbobrDispatcher};

impl ZbobrDispatcher {
    /// Clean up workspaces directories for closed tasks.
    /// If dry_run is true, only logs what would happen.
    pub async fn cleanup_closed_tasks(&self, backends: &Backends, dry_run: bool) -> anyhow::Result<()> {
        let workspaces = &self.config().workspaces;

        if !workspaces.exists() {
            tracing::info!(
                "Workspaces directory does not exist: {}",
                workspaces.display()
            );
            return Ok(());
        }

        tracing::info!("Scanning workspaces: {}", workspaces.display());
        if dry_run {
            tracing::info!("DRY RUN - no files will be deleted");
        }

        let mut entries = tokio::fs::read_dir(workspaces).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Try to parse as a TaskDir
            let task_dir = match TaskDir::from_path(workspaces, &path) {
                Ok(td) => td,
                Err(_) => {
                    // Not a task directory, skip
                    continue;
                }
            };

            let task_id = task_dir.task_id();

            // Check if task is closed by trying to get it. If get_task fails,
            // the task was deleted/closed and we can clean up the workspace.
            match backends.tasks().get_task(task_id).await {
                Ok(weak) => {
                    match weak.snapshot().await {
                        Ok(task) if task.stage == crate::Stage::Done => {
                            if dry_run {
                                tracing::info!(
                                    "DRY RUN: Would remove {} (task #{task_id} is DONE)",
                                    path.display()
                                );
                            } else {
                                tracing::info!("Removing {} (task #{task_id} is DONE)", path.display());
                                tokio::fs::remove_dir_all(&path).await?;
                            }
                        }
                        Ok(_) => {
                            tracing::info!(
                                "Task #{task_id} is open - keeping {}",
                                entry.path().display()
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to read task #{task_id}: {e} - skipping");
                        }
                    }
                }
                Err(_) => {
                    // Task not found — likely closed/deleted
                    if dry_run {
                        tracing::info!(
                            "DRY RUN: Would remove {} (task #{task_id} not found)",
                            path.display()
                        );
                    } else {
                        tracing::info!("Removing {} (task #{task_id} not found)", path.display());
                        tokio::fs::remove_dir_all(&path).await?;
                    }
                }
            }
        }

        tracing::info!("Cleanup complete");
        Ok(())
    }
}
