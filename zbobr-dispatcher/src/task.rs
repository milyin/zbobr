pub use zbobr_api::task::*;

use std::sync::Arc;

use crate::{
    TaskDir, ZbobrDispatcher,
};

// ---------------------------------------------------------------------------
// RoleSession — restricted access for MCP tools during agent sessions.
//
// Cannot modify: stage, conflict, confirm (those are dispatcher-only transitions).
// Can modify: description, checklist, parameters, signal, pause.
// ---------------------------------------------------------------------------

/// Restricted task session for MCP tool operations.
/// State and stack are protected — only the dispatcher may change them.
#[derive(Clone)]
pub struct RoleSession {
    zbobr: Arc<ZbobrDispatcher>,
    task_id: u64,
    /// Tracks the last MCP tool call that matched a transition key.
    last_mapped_tool: Arc<std::sync::Mutex<Option<String>>>,
    /// Pipeline name for this session's comments.
    pipeline_name: String,
    /// Pipeline run ID for this session's comments.
    pipeline_run_id: u64,
}

impl RoleSession {
    const CHECKLIST_SCOPE_DELIMITER: &str = "__";

    pub(crate) fn new(zbobr: Arc<ZbobrDispatcher>, task_id: u64) -> Self {
        Self {
            zbobr,
            task_id,
            last_mapped_tool: Arc::new(std::sync::Mutex::new(None)),
            pipeline_name: String::new(),
            pipeline_run_id: 0,
        }
    }

    pub(crate) fn with_shared_tracker(
        zbobr: Arc<ZbobrDispatcher>,
        task_id: u64,
        tracker: Arc<std::sync::Mutex<Option<String>>>,
        pipeline_name: String,
        pipeline_run_id: u64,
    ) -> Self {
        Self {
            zbobr,
            task_id,
            last_mapped_tool: tracker,
            pipeline_name,
            pipeline_run_id,
        }
    }

    pub fn task_id(&self) -> u64 {
        self.task_id
    }

    /// Create a branch name with the proper prefix for this task.
    pub fn create_branch_name(&self, short_name: &str) -> String {
        format!(
            "{}-{}-{}",
            self.zbobr.config().work_branch_prefix,
            self.task_id,
            short_name
        )
    }

    /// Check whether a branch name starts with this task's prefix.
    pub fn validate_branch_prefix(&self, branch: &str) -> bool {
        let prefix = format!(
            "{}-{}-",
            self.zbobr.config().work_branch_prefix,
            self.task_id
        );
        branch.starts_with(&prefix)
    }

    /// Read the full task state.
    pub async fn get_task(&self) -> anyhow::Result<Task> {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        weak.snapshot().await
    }

    /// Get the current task description.
    pub async fn get_description(&self) -> anyhow::Result<String> {
        Ok(self.get_task().await?.description)
    }

    /// Get the full discussion history for the current pipeline run.
    pub async fn get_history_for_run(&self, target_run_id: u64) -> anyhow::Result<String> {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        let comments = weak.get_comments().await?;
        let task = weak.snapshot().await?;
        let filtered = zbobr_api::filter_comments_for_run(&comments, target_run_id);
        let mut parts = vec![format!("[task]\n{}", task.description)];
        for comment in filtered {
            parts.push(format!("[{}]\n{}", comment.stage, comment.text));
        }
        Ok(parts.join("\n\n---\n\n"))
    }

    /// Get pipeline name for this session.
    pub fn pipeline_name(&self) -> &str {
        &self.pipeline_name
    }

    /// Get pipeline run ID for this session.
    pub fn pipeline_run_id(&self) -> u64 {
        self.pipeline_run_id
    }

    /// Get the current task checklist.
    pub async fn get_checklist(&self) -> anyhow::Result<Vec<ChecklistItem>> {
        let task = self.get_task().await?;
        let Some(prefix) = self.checklist_scope_prefix() else {
            return Ok(task.checklist);
        };

        let scoped = task
            .checklist
            .into_iter()
            .filter_map(|item| Self::strip_checklist_scope(item, &prefix))
            .collect();
        Ok(scoped)
    }

    /// Add a checklist item scoped to the current pipeline run.
    pub async fn add_checklist_item(&self, id: &str, text: &str) -> anyhow::Result<()> {
        let item_id = id.to_string();
        let item_text = text.to_string();
        let scope_prefix = self.checklist_scope_prefix();

        self.modify_task(move |mut task| {
            let scoped_id = if let Some(ref prefix) = scope_prefix {
                format!("{prefix}{item_id}")
            } else {
                item_id
            };
            task.checklist.push(ChecklistItem {
                id: scoped_id,
                checked: false,
                text: item_text,
            });
            task
        })
        .await
    }

    /// Mark a scoped checklist item as checked.
    /// Returns true when an item was found and updated.
    pub async fn check_checklist_item(&self, id: &str) -> anyhow::Result<bool> {
        let item_id = id.to_string();
        let scope_prefix = self.checklist_scope_prefix();

        let found = Arc::new(std::sync::Mutex::new(false));
        let found_ref = Arc::clone(&found);

        self.modify_task(move |mut task| {
            let target_id = if let Some(ref prefix) = scope_prefix {
                format!("{prefix}{item_id}")
            } else {
                item_id
            };

            if let Some(item) = task.checklist.iter_mut().find(|item| item.id == target_id) {
                item.checked = true;
                *found_ref.lock().unwrap() = true;
            }
            task
        })
        .await?;

        Ok(*found.lock().unwrap())
    }

    /// Delete a scoped checklist item.
    /// Returns true when an item was removed.
    pub async fn delete_checklist_item(&self, id: &str) -> anyhow::Result<bool> {
        let item_id = id.to_string();
        let scope_prefix = self.checklist_scope_prefix();

        let removed = Arc::new(std::sync::Mutex::new(false));
        let removed_ref = Arc::clone(&removed);

        self.modify_task(move |mut task| {
            let target_id = if let Some(ref prefix) = scope_prefix {
                format!("{prefix}{item_id}")
            } else {
                item_id
            };

            let before = task.checklist.len();
            task.checklist.retain(|item| item.id != target_id);
            *removed_ref.lock().unwrap() = task.checklist.len() != before;
            task
        })
        .await?;

        Ok(*removed.lock().unwrap())
    }

    fn checklist_scope_prefix(&self) -> Option<String> {
        if self.pipeline_name.is_empty() || self.pipeline_run_id == 0 {
            return None;
        }
        Some(format!(
            "{}{}{}{}",
            self.pipeline_name,
            Self::CHECKLIST_SCOPE_DELIMITER,
            self.pipeline_run_id,
            Self::CHECKLIST_SCOPE_DELIMITER
        ))
    }

    fn strip_checklist_scope(item: ChecklistItem, scope_prefix: &str) -> Option<ChecklistItem> {
        item.id.strip_prefix(scope_prefix).map(|id| ChecklistItem {
            id: id.to_string(),
            checked: item.checked,
            text: item.text,
        })
    }

    /// Atomically read-modify-write the task body via transient upgrade.
    ///
    /// The closure receives a mutable `Task` reference and may modify `description`,
    /// `parameters`, `checklist`, `signal`, and `pause`.
    ///
    /// **Protected fields**: `state` and `stack` are saved before the mutation
    /// and restored afterwards, so MCP tools cannot change them.
    pub async fn modify_task<F>(&self, mutate: F) -> anyhow::Result<()>
    where
        F: FnOnce(Task) -> Task + Send + 'static,
    {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        let mutable = weak.upgrade().await?;
        mutable
            .modify_task(Box::new(move |mut task| {
                let saved_state = task.state.clone();
                let saved_stack = task.stack.clone();
                task = mutate(task);
                task.state = saved_state;
                task.stack = saved_stack;
                task
            }))
            .await
    }

    /// Get all comments as structured `Comment` objects.
    pub async fn get_comments(&self) -> anyhow::Result<Vec<Comment>> {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        weak.get_comments().await
    }

    /// Post a comment immediately to the backend.
    pub async fn post_comment(
        &self,
        body: &str,
        stage: &str,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
        report_text: Option<&str>,
    ) -> anyhow::Result<()> {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        let mutable = weak.upgrade().await?;
        mutable
            .post_comment(
                stage,
                hostname,
                tool,
                model,
                body,
                &self.pipeline_name,
                self.pipeline_run_id,
                None,
                None,
                report_text,
            )
            .await
    }

    /// Get the current signal on the task.
    pub async fn get_signal(&self) -> anyhow::Result<Option<Signal>> {
        let task = self.get_task().await?;
        Ok(task.signal)
    }

    /// Set signal on the task.
    pub async fn set_signal(&self, new_signal: Signal) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.signal = Some(new_signal);
            task
        })
        .await
    }

    /// Clear the signal on the task.
    pub async fn clear_signal(&self) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.signal = None;
            task
        })
        .await
    }

    /// Set the pause flag on the task.
    pub async fn set_pause(&self, pause: bool) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.pause = pause;
            task
        })
        .await
    }

    /// Get the destination_repository field.
    pub async fn get_destination_repository(&self) -> anyhow::Result<Option<String>> {
        let task = self.get_task().await?;
        Ok(task.destination_repository)
    }

    /// Set the destination_repository field.
    pub async fn set_destination_repository(&self, value: Option<String>) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.destination_repository = value;
            task
        })
        .await
    }

    /// Get the destination_branch field.
    pub async fn get_destination_branch(&self) -> anyhow::Result<Option<String>> {
        let task = self.get_task().await?;
        Ok(task.destination_branch)
    }

    /// Set the destination_branch field.
    pub async fn set_destination_branch(&self, value: Option<String>) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.destination_branch = value;
            task
        })
        .await
    }

    /// Get the work_branch field.
    pub async fn get_work_branch(&self) -> anyhow::Result<Option<String>> {
        let task = self.get_task().await?;
        Ok(task.work_branch)
    }

    /// Set the work_branch field.
    pub async fn set_work_branch(&self, value: Option<String>) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.work_branch = value;
            task
        })
        .await
    }

    /// Read a report file via the task backend.
    pub async fn read_report(&self, name: &str) -> anyhow::Result<String> {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        weak.read_report(name).await
    }

    /// Record a tool call for transition mapping.
    /// Only `report_success` and `report_failure` are meaningful transition triggers.
    pub fn record_tool_call(&self, tool_name: &str) {
        if tool_name == "report_success" || tool_name == "report_failure" {
            *self.last_mapped_tool.lock().unwrap() = Some(tool_name.to_string());
        }
    }

    /// Get the last MCP tool call that matched a transition key.
    pub fn last_mapped_tool(&self) -> Option<String> {
        self.last_mapped_tool.lock().unwrap().clone()
    }

}

// ---------------------------------------------------------------------------
// TaskSession — full access for the dispatcher.
//
// Can modify everything including stage and conflict flag.
// ---------------------------------------------------------------------------

/// Full-access task session for the dispatcher.
/// Can change stage, conflict flag, and all other fields.
#[derive(Clone)]
pub struct TaskSession {
    zbobr: Arc<ZbobrDispatcher>,
    task_id: u64,
}

impl TaskSession {
    pub(crate) fn new(
        zbobr: Arc<ZbobrDispatcher>,
        task_id: u64,
    ) -> Self {
        Self {
            zbobr,
            task_id,
        }
    }

    pub fn task_id(&self) -> u64 {
        self.task_id
    }

    /// Get a restricted RoleSession view for MCP tool operations.
    pub fn role_session(&self) -> RoleSession {
        RoleSession::new(Arc::clone(&self.zbobr), self.task_id)
    }

    /// Read the full task state.
    pub async fn get_task(&self) -> anyhow::Result<Task> {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        weak.snapshot().await
    }

    /// Get the current task checklist.
    pub async fn get_checklist(&self) -> anyhow::Result<Vec<ChecklistItem>> {
        Ok(self.get_task().await?.checklist)
    }

    /// Atomically read-modify-write the task with unrestricted access via transient upgrade.
    pub async fn modify_task<F>(&self, mutate: F) -> anyhow::Result<()>
    where
        F: FnOnce(Task) -> Task + Send + 'static,
    {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        let mutable = weak.upgrade().await?;
        mutable
            .modify_task(Box::new(move |mut task| {
                task = mutate(task);
                task
            }))
            .await
    }

    /// Set the task state (dispatcher only).
    pub async fn set_state(&self, state: &str) -> anyhow::Result<()> {
        let state = state.to_string();
        self.modify_task(move |mut task| {
            if task.confirm && task.state != state {
                task.pause = true;
            }
            task.state = state;
            task
        })
        .await
    }

    /// Set the confirm flag on the task (dispatcher only).
    pub async fn set_confirm(&self, confirm: bool) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.confirm = confirm;
            task
        })
        .await
    }

    /// Set signal on the task (dispatcher only).
    pub async fn set_signal(&self, signal: Option<Signal>) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.signal = signal;
            task
        })
        .await
    }

    /// Allocate a new pipeline run ID (monotonically incrementing).
    pub async fn allocate_pipeline_run_id(&self) -> anyhow::Result<u64> {
        let task = self.get_task().await?;
        let new_id = task.pipeline_run_id + 1;
        self.modify_task(move |mut t| {
            t.pipeline_run_id = new_id;
            t
        })
        .await?;
        Ok(new_id)
    }

    /// Push an entry onto the task's call stack, saving the current pipeline_run_id.
    pub async fn push_stack(&self, pipeline: impl Into<crate::task::Pipeline>, signal: Signal) -> anyhow::Result<()> {
        let task = self.get_task().await?;
        let entry = crate::task::StackEntry {
            pipeline: pipeline.into(),
            signal,
            pipeline_run_id: task.pipeline_run_id,
        };
        self.modify_task(move |mut task| {
            task.stack.push(entry);
            task
        })
        .await
    }

    /// Pop the top entry from the task's call stack. Restores pipeline_run_id.
    pub async fn pop_stack(&self) -> anyhow::Result<Option<crate::task::StackEntry>> {
        let task = self.get_task().await?;
        let popped = task.stack.last().cloned();
        if let Some(ref entry) = popped {
            let restored_run_id = entry.pipeline_run_id;
            self.modify_task(move |mut task| {
                task.stack.pop();
                task.pipeline_run_id = restored_run_id;
                task
            })
            .await?;
        }
        Ok(popped)
    }

    /// Finish the task: delete placeholder commit, push branch, post Done comment,
    /// then set state to DONE and clear signal.
    pub async fn finish(&self) -> anyhow::Result<()> {
        let task_id = self.task_id;
        let task = self.get_task().await?;

        // Delete placeholder commit and push before marking done.
        if let Some(work_branch) = &task.work_branch {
            let task_dir = TaskDir::new(self.zbobr.config().workspaces.as_path(), task_id);
            let work_dir = if let Some(ref dest_repo) = task.destination_repository {
                let repo_name = dest_repo.rsplit('/').next().unwrap_or(dest_repo.as_str());
                task_dir.path().join(repo_name)
            } else {
                task_dir.path().to_path_buf()
            };
            if let Err(e) = zbobr_utility::delete_placeholder_commit(&work_dir, work_branch).await {
                tracing::warn!("Failed to delete placeholder commit for task #{task_id}: {e}");
            } else if let Some(identity) = task.identity() {
                if let Err(e) = self.zbobr.repo_backend().update_pr(&identity).await {
                    tracing::warn!(
                        "Failed to push branch after placeholder deletion for task #{task_id}: {e}"
                    );
                }
            }
        }

        // Post DONE marker comment.
        let hostname = crate::mcp::common::get_hostname();
        let task = self.get_task().await?;
        if let Err(e) = self
            .post_comment(
                "done",
                &hostname,
                None,
                None,
                "",
                "",
                task.pipeline_run_id,
                None,
                None,
            )
            .await
        {
            tracing::warn!("Failed to post DONE boundary for task #{task_id}: {e}");
        }

        self.modify_task(move |mut task| {
            task.state = "DONE".to_string();
            task.signal = None;
            task
        })
        .await
    }

    /// Post a structured comment directly to the backend.
    pub async fn post_comment(
        &self,
        stage: &str,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
        body: &str,
        pipeline: &str,
        pipeline_run_id: u64,
        caller_pipeline: Option<&str>,
        caller_pipeline_run_id: Option<u64>,
    ) -> anyhow::Result<()> {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        let mutable = weak.upgrade().await?;
        mutable
            .post_comment(
                stage,
                hostname,
                tool,
                model,
                body,
                pipeline,
                pipeline_run_id,
                caller_pipeline,
                caller_pipeline_run_id,
                None,
            )
            .await
    }
}

#[cfg(test)]
mod comment_model_tests {
    use super::*;
    use crate::config::ZbobrDispatcherConfig;
    use crate::mcp::traits::CommonMcpImpl;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, atomic::AtomicU64};
    use tokio::sync::Mutex;
    use zbobr_api::backend::{TaskBackend, TaskMut, TaskWeak};

    // Simple in-memory backend for testing

    struct InMemTask {
        task: Task,
        closed: bool,
    }

    struct TrackingBackend {
        tasks: Mutex<HashMap<u64, InMemTask>>,
        comments: Mutex<HashMap<u64, Vec<Comment>>>,
        /// In-memory report storage: (task_id, filename) -> content
        reports: Mutex<HashMap<(u64, String), String>>,
        next_id: AtomicU64,
        locks: Mutex<HashMap<u64, Arc<Mutex<()>>>>,
    }

    struct TrackingWeak {
        id: u64,
        backend: Arc<TrackingBackend>,
    }

    struct TrackingMut {
        id: u64,
        backend: Arc<TrackingBackend>,
        _guard: tokio::sync::OwnedMutexGuard<()>,
    }

    impl TrackingBackend {
        async fn task_lock(&self, id: u64) -> Arc<Mutex<()>> {
            let mut locks = self.locks.lock().await;
            locks
                .entry(id)
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        }
    }

    #[async_trait]
    impl TaskWeak for TrackingWeak {
        fn task_id(&self) -> u64 {
            self.id
        }

        async fn snapshot(&self) -> anyhow::Result<Task> {
            let tasks = self.backend.tasks.lock().await;
            tasks
                .get(&self.id)
                .map(|t| t.task.clone())
                .ok_or_else(|| anyhow::anyhow!("not found"))
        }

        async fn upgrade(&self) -> anyhow::Result<Box<dyn TaskMut>> {
            let lock = self.backend.task_lock(self.id).await;
            let guard = lock
                .try_lock_owned()
                .map_err(|_| anyhow::anyhow!("locked"))?;
            Ok(Box::new(TrackingMut {
                id: self.id,
                backend: self.backend.clone(),
                _guard: guard,
            }))
        }

        async fn get_comments(&self) -> anyhow::Result<Vec<Comment>> {
            let comments = self.backend.comments.lock().await;
            Ok(comments.get(&self.id).cloned().unwrap_or_default())
        }

        async fn read_report(&self, name: &str) -> anyhow::Result<String> {
            let reports = self.backend.reports.lock().await;
            reports
                .get(&(self.id, name.to_string()))
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("report not found: {name}"))
        }
    }

    #[async_trait]
    impl TaskMut for TrackingMut {
        fn task_id(&self) -> u64 {
            self.id
        }

        async fn snapshot(&self) -> anyhow::Result<Task> {
            let tasks = self.backend.tasks.lock().await;
            tasks
                .get(&self.id)
                .map(|t| t.task.clone())
                .ok_or_else(|| anyhow::anyhow!("not found"))
        }

        async fn modify_task(
            &self,
            mutate: Box<dyn FnOnce(Task) -> Task + Send>,
        ) -> anyhow::Result<()> {
            let mut tasks = self.backend.tasks.lock().await;
            if let Some(t) = tasks.get_mut(&self.id) {
                let task = t.task.clone();
                t.task = mutate(task);
                Ok(())
            } else {
                Err(anyhow::anyhow!("not found"))
            }
        }

        async fn close(&self) -> anyhow::Result<()> {
            let mut tasks = self.backend.tasks.lock().await;
            if let Some(t) = tasks.get_mut(&self.id) {
                t.closed = true;
            }
            Ok(())
        }

        async fn post_comment(
            &self,
            stage: &str,
            hostname: &str,
            tool: Option<Tool>,
            model: Option<Model>,
            body: &str,
            pipeline: &str,
            pipeline_run_id: u64,
            caller_pipeline: Option<&str>,
            caller_pipeline_run_id: Option<u64>,
            report_text: Option<&str>,
        ) -> anyhow::Result<()> {
            let report_name = if let Some(text) = report_text {
                let tag = match classify_comment(body) {
                    HistoryRecordType::Success => "success",
                    HistoryRecordType::Failure => "failure",
                    _ => "report",
                };
                let base_name = format!("report_{pipeline}_{pipeline_run_id}_{stage}_{tag}");
                let mut reports = self.backend.reports.lock().await;
                let mut n = 0u32;
                let filename = loop {
                    let candidate = if n == 0 {
                        format!("{base_name}.md")
                    } else {
                        format!("{base_name}_{n}.md")
                    };
                    if !reports.contains_key(&(self.id, candidate.clone())) {
                        break candidate;
                    }
                    n += 1;
                };
                reports.insert((self.id, filename.clone()), text.to_string());
                Some(filename)
            } else {
                None
            };

            let mut comments = self.backend.comments.lock().await;
            comments.entry(self.id).or_default().push(Comment {
                timestamp: String::new(),
                stage: stage.to_string(),
                hostname: hostname.to_string(),
                tool,
                model,
                text: body.to_string(),
                pipeline: pipeline.to_string(),
                pipeline_run_id,
                caller_pipeline: caller_pipeline.map(str::to_string),
                caller_pipeline_run_id,
                report_name,
            });
            Ok(())
        }

        fn downgrade(self: Box<Self>) -> Box<dyn TaskWeak> {
            Box::new(TrackingWeak {
                id: self.id,
                backend: self.backend.clone(),
            })
        }
    }

    #[derive(Clone)]
    struct ArcTrackingBackend {
        inner: Arc<TrackingBackend>,
    }

    #[async_trait]
    impl TaskBackend for ArcTrackingBackend {
        async fn get_task(&self, id: u64) -> anyhow::Result<Box<dyn TaskWeak>> {
            let tasks = self.inner.tasks.lock().await;
            if tasks.contains_key(&id) {
                Ok(Box::new(TrackingWeak {
                    id,
                    backend: self.inner.clone(),
                }))
            } else {
                Err(anyhow::anyhow!("not found"))
            }
        }

        async fn list_tasks(
            &self,
        ) -> anyhow::Result<Vec<Box<dyn TaskWeak>>> {
            Ok(vec![])
        }

        async fn create_task(
            &self,
            title: &str,
            description: &str,
            state: &str,
        ) -> anyhow::Result<u64> {
            let id = self
                .inner
                .next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            let task = Task {
                id,
                title: title.to_string(),
                description: description.to_string(),
                state: state.to_string(),
                destination_repository: None,
                destination_branch: None,
                work_branch: None,
                pr_url: None,
                checklist: vec![],
                signal: None,
                stack: vec![],
                pause: false,
                confirm: false,
                pipeline_run_id: 0,
                etag: None,
            };
            self.inner.tasks.lock().await.insert(
                id,
                InMemTask {
                    task,
                    closed: false,
                },
            );
            Ok(id)
        }

        async fn setup(&self, _force: bool) -> anyhow::Result<()> {
            Ok(())
        }
        async fn validate_connectivity(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn debug_state(&self) -> String {
            "tracking".to_string()
        }
    }

    #[derive(Clone)]
    struct DummyRepo;

    #[async_trait]
    impl crate::backend::WorktreeBackend for DummyRepo {
        async fn update_worktree(
            &self,
            _identity: &zbobr_api::TaskIdentity,
            _workspace_path: &std::path::Path,
            _git_user_name: &str,
            _git_user_email: &str,
        ) -> anyhow::Result<bool> {
            Ok(true)
        }

        async fn update_pr(&self, _identity: &zbobr_api::TaskIdentity) -> anyhow::Result<String> {
            Ok("mock-pr-url".to_string())
        }

        async fn validate_connectivity(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn debug_state(&self) -> String {
            "dummy".to_string()
        }
    }

    fn make_test_parts() -> (Arc<crate::ZbobrDispatcher>, ArcTrackingBackend) {
        let backend = ArcTrackingBackend {
            inner: Arc::new(TrackingBackend {
                tasks: Mutex::new(HashMap::new()),
                comments: Mutex::new(HashMap::new()),
                reports: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(0),
                locks: Mutex::new(HashMap::new()),
            }),
        };
        let zbobr = Arc::new(crate::ZbobrDispatcherBuilder::new()
            .with_config(ZbobrDispatcherConfig::default())
            .with_workflow(crate::workflow::Workflow::default())
            .with_task_backend(backend.clone())
            .with_repo_backend(DummyRepo)
            .build());
        (zbobr, backend)
    }

    #[tokio::test]
    async fn mcp_helper_includes_explicit_model() {
        let (zbobr, task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "", "READY", None, None)
            .await
            .unwrap();

        let session = zbobr.role_session(id);
        let allowed_tools: std::collections::HashSet<String> =
            ["get_history", "stop_with_error", "report_success"]
                .iter()
                .map(|s| s.to_string())
                .collect();
        let planner = crate::mcp::unified::UnifiedMcp::new(
            session,
            allowed_tools,
            "planner".to_string(),
            Tool::Copilot,
            Model::Gpt5Mini,
            "planning".to_string(),
            "main".to_string(),
            1,
        );

        // stop_with_error is unbuffered — goes straight to backend
        let _ = planner.stop_with_error_impl("oops").await;

        let weak = task_backend.get_task(id).await.unwrap();
        let comments = weak.get_comments().await.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].model, Some(Model::Gpt5Mini));
        assert_eq!(comments[0].tool, Some(Tool::Copilot));
        assert!(comments[0].text.starts_with("[stop_with_error]"));
    }

    #[tokio::test]
    async fn dispatcher_posts_have_no_model() {
        let (zbobr, task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "", "READY", None, None)
            .await
            .unwrap();

        zbobr
            .task_session(id)
            .post_comment(
                "error",
                "host",
                None,
                None,
                "dispatcher error",
                "",
                0,
                None,
                None,
            )
            .await
            .unwrap();

        let weak = task_backend.get_task(id).await.unwrap();
        let comments = weak.get_comments().await.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].model, None);
        assert_eq!(comments[0].tool, None);
    }

    #[test]
    fn comment_tag_roundtrip() {
        let tag = CommentTag::new(
            "main".to_string(),
            3,
            "planning".to_string(),
            "host".to_string(),
            Some(Tool::Copilot),
            Some(Model::Gpt5Mini),
        );
        assert_eq!(tag.to_string(), "// main:3:planning by host:copilot:gpt-5-mini");
        let parsed: CommentTag = tag.to_string().parse().unwrap();
        assert_eq!(parsed, tag);

        let tag_no_tool = CommentTag::new(
            "main".to_string(),
            1,
            "done".to_string(),
            "web".to_string(),
            None,
            None,
        );
        assert_eq!(tag_no_tool.to_string(), "// main:1:done by web");
        let parsed_no_tool: CommentTag = tag_no_tool.to_string().parse().unwrap();
        assert_eq!(parsed_no_tool, tag_no_tool);

        let tag_tool_no_model = CommentTag::new(
            "init".to_string(),
            2,
            "working".to_string(),
            "host".to_string(),
            Some(Tool::Claude),
            None,
        );
        assert_eq!(tag_tool_no_model.to_string(), "// init:2:working by host:claude");
        let parsed_tnm: CommentTag = tag_tool_no_model.to_string().parse().unwrap();
        assert_eq!(parsed_tnm, tag_tool_no_model);
    }

    // -----------------------------------------------------------------------
    // Helper: create a UnifiedMcp for tests
    // -----------------------------------------------------------------------

    fn make_test_mcp(
        zbobr: &Arc<crate::ZbobrDispatcher>,
        task_id: u64,
    ) -> crate::mcp::unified::UnifiedMcp {
        let tracker = Arc::new(std::sync::Mutex::new(None::<String>));
        let session = zbobr.role_session_with_tracker(
            task_id,
            tracker,
            "main".to_string(),
            1,
        );
        let allowed_tools: std::collections::HashSet<String> = crate::mcp::unified::ALL_TOOL_NAMES
            .iter()
            .map(|s| s.to_string())
            .collect();
        crate::mcp::unified::UnifiedMcp::new(
            session,
            allowed_tools,
            "worker".to_string(),
            Tool::Copilot,
            Model::Gpt5Mini,
            "working".to_string(),
            "main".to_string(),
            1,
        )
    }

    #[tokio::test]
    async fn report_success_posts_comment_to_backend() {
        let (zbobr, task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "desc", "READY", None, None)
            .await
            .unwrap();

        let mcp = make_test_mcp(&zbobr, id);

        let _ = mcp.report_success_impl("result one", "detailed success report").await;
        let _ = mcp.report_failure_impl("needs work", "detailed failure report").await;

        let weak = task_backend.get_task(id).await.unwrap();
        let backend_comments = weak.get_comments().await.unwrap();
        assert_eq!(backend_comments.len(), 2, "each comment must be posted separately");
        assert!(backend_comments[0].text.starts_with("[report_success]"));
        assert_eq!(backend_comments[0].report_name.as_deref(), Some("report_main_1_working_success.md"));
        assert!(backend_comments[1].text.starts_with("[report_failure]"));
        assert_eq!(backend_comments[1].report_name.as_deref(), Some("report_main_1_working_failure.md"));

        // Verify reports were stored and are readable
        let success_report = weak.read_report("report_main_1_working_success.md").await.unwrap();
        assert_eq!(success_report, "detailed success report");
        let failure_report = weak.read_report("report_main_1_working_failure.md").await.unwrap();
        assert_eq!(failure_report, "detailed failure report");
    }

    #[tokio::test]
    async fn checklist_is_scoped_by_pipeline_run_id() {
        let (zbobr, task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "desc", "READY", None, None)
            .await
            .unwrap();

        let tracker_a = Arc::new(std::sync::Mutex::new(None::<String>));
        let session_a = zbobr.role_session_with_tracker(
            id,
            tracker_a,
            "main".to_string(),
            1,
        );

        let tracker_b = Arc::new(std::sync::Mutex::new(None::<String>));
        let session_b = zbobr.role_session_with_tracker(
            id,
            tracker_b,
            "main".to_string(),
            2,
        );

        session_a.add_checklist_item("same-id", "run-1 item").await.unwrap();
        session_b.add_checklist_item("same-id", "run-2 item").await.unwrap();

        let run1_items = session_a.get_checklist().await.unwrap();
        assert_eq!(run1_items.len(), 1);
        assert_eq!(run1_items[0].id, "same-id");
        assert_eq!(run1_items[0].text, "run-1 item");
        assert!(!run1_items[0].checked);

        let run2_items = session_b.get_checklist().await.unwrap();
        assert_eq!(run2_items.len(), 1);
        assert_eq!(run2_items[0].id, "same-id");
        assert_eq!(run2_items[0].text, "run-2 item");
        assert!(!run2_items[0].checked);

        session_a.check_checklist_item("same-id").await.unwrap();

        let run1_after_check = session_a.get_checklist().await.unwrap();
        let run2_after_check = session_b.get_checklist().await.unwrap();
        assert!(run1_after_check[0].checked);
        assert!(!run2_after_check[0].checked);

        let weak = task_backend.get_task(id).await.unwrap();
        let task = weak.snapshot().await.unwrap();
        assert_eq!(task.checklist.len(), 2);
        assert!(task.checklist.iter().any(|i| i.id == "main__1__same-id"));
        assert!(task.checklist.iter().any(|i| i.id == "main__2__same-id"));
    }

    #[tokio::test]
    async fn checklist_without_pipeline_context_remains_unscoped() {
        let (zbobr, task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "desc", "READY", None, None)
            .await
            .unwrap();

        let session = zbobr.role_session(id);
        session.add_checklist_item("plain", "legacy item").await.unwrap();

        let items = session.get_checklist().await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "plain");
        assert_eq!(items[0].text, "legacy item");

        let weak = task_backend.get_task(id).await.unwrap();
        let task = weak.snapshot().await.unwrap();
        assert_eq!(task.checklist.len(), 1);
        assert_eq!(task.checklist[0].id, "plain");
    }
}
