use std::sync::Arc;

pub use zbobr_api::task::*;

use zbobr_api::config::ZbobrDispatcherConfig;
use zbobr_api::config_tools::McpTool;

use crate::{TaskDir, ZbobrDispatcher};

// ---------------------------------------------------------------------------
// RoleSession — restricted access for MCP tools during agent sessions.
//
// Cannot modify: stage, conflict, confirm (those are dispatcher-only transitions).
// Can modify: description, context, parameters, signal, pause.
// ---------------------------------------------------------------------------

/// Restricted task session for MCP tool operations.
/// State and stack are protected — only the dispatcher may change them.
#[derive(Clone)]
pub struct RoleSession {
    zbobr: Arc<ZbobrDispatcher>,
    task_id: u64,
    /// Tracks the last MCP tool call that matched a transition key.
    last_mapped_tool: Arc<std::sync::Mutex<Option<McpTool>>>,
    /// Pipeline name for this session's comments.
    pipeline_name: String,
    /// Pipeline run ID for this session's comments.
    pipeline_run_id: u64,
    /// Holds the prompt text for logging alongside comments.
    prompt_text: Arc<std::sync::Mutex<Option<String>>>,
}

impl RoleSession {
    pub(crate) fn new(zbobr: Arc<ZbobrDispatcher>, task_id: u64) -> Self {
        Self {
            zbobr,
            task_id,
            last_mapped_tool: Arc::new(std::sync::Mutex::new(None)),
            pipeline_name: String::new(),
            pipeline_run_id: 0,
            prompt_text: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub(crate) fn with_shared_tracker(
        zbobr: Arc<ZbobrDispatcher>,
        task_id: u64,
        tracker: Arc<std::sync::Mutex<Option<McpTool>>>,
        pipeline_name: String,
        pipeline_run_id: u64,
        prompt_holder: Arc<std::sync::Mutex<Option<String>>>,
    ) -> Self {
        Self {
            zbobr,
            task_id,
            last_mapped_tool: tracker,
            pipeline_name,
            pipeline_run_id,
            prompt_text: prompt_holder,
        }
    }

    pub fn task_id(&self) -> u64 {
        self.task_id
    }

    pub fn dispatcher_config(&self) -> &ZbobrDispatcherConfig {
        self.zbobr.config()
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
        weak.snapshot(false).await
    }

    /// Get the current task description.
    pub async fn get_description(&self) -> anyhow::Result<String> {
        Ok(self.get_task().await?.description)
    }

    /// Get pipeline name for this session.
    pub fn pipeline_name(&self) -> &str {
        &self.pipeline_name
    }

    /// Get pipeline run ID for this session.
    pub fn pipeline_run_id(&self) -> u64 {
        self.pipeline_run_id
    }

    /// Atomically read-modify-write the task body via transient upgrade.
    ///
    /// The closure receives a mutable `Task` reference and may modify `description`,
    /// `parameters`, `context`, `signal`, and `pause`.
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
                let saved_stage_count = task.stage_count;
                let saved_max_stage_count = task.max_stage_count;
                task = mutate(task);
                task.state = saved_state;
                task.stack = saved_stack;
                task.stage_count = saved_stage_count;
                task.max_stage_count = saved_max_stage_count;
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
        let prompt = self.prompt_text.lock().unwrap().clone();
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
                prompt.as_deref(),
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

    /// Set pause to true and assign a signal in a single modify_task call.
    pub async fn set_pause_with_signal(&self, signal: Signal) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.pause = true;
            task.signal = Some(signal);
            task
        })
        .await
    }

    /// Set the error message on the task.
    pub async fn set_error(&self, error: Option<String>) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.error = error;
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

    /// Store a report file via the task backend. Returns the stored filename.
    pub async fn store_report(&self, base_name: &str, content: &str) -> anyhow::Result<String> {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        let mutable = weak.upgrade().await?;
        mutable.store_report(base_name, content).await
    }

    /// Add a checkbox context record to the current stage.
    /// Returns the assigned record id.
    pub async fn add_checkbox_record(
        &self,
        brief: String,
        report_link: Option<String>,
        parent_record_id: Option<u64>,
    ) -> anyhow::Result<u64> {
        self.modify_task(move |mut task| {
            let id = task.context.next_id();
            if let Some(stage) = task.context.stages.last_mut() {
                stage.records.push(ContextRecord {
                    id,
                    record_type: ContextRecordType::Checkbox(false),
                    brief,
                    report_link,
                    parent_record_id,
                });
            }
            task
        })
        .await?;
        // Re-read to get the actual assigned id
        let task = self.get_task().await?;
        let last_id = task
            .context
            .stages
            .last()
            .and_then(|s| s.records.last())
            .map(|r| r.id)
            .unwrap_or(0);
        Ok(last_id)
    }

    /// Check (mark as done) a checkbox context record by id.
    pub async fn check_checkbox_record(&self, record_id: u64) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            if let Some((_, record)) = task.context.find_record_mut(record_id) {
                if let ContextRecordType::Checkbox(_) = record.record_type {
                    record.record_type = ContextRecordType::Checkbox(true);
                }
            }
            task
        })
        .await
    }

    /// Delete a context record by id.
    pub async fn delete_context_record(&self, record_id: u64) -> anyhow::Result<bool> {
        let task_before = self.get_task().await?;
        let exists = task_before.context.find_record(record_id).is_some();
        if exists {
            self.modify_task(move |mut task| {
                task.context.delete_record(record_id);
                task
            })
            .await?;
        }
        Ok(exists)
    }

    /// Add a context record of a specific type to the current stage.
    /// Returns the assigned record id.
    pub async fn add_context_record(
        &self,
        record_type: ContextRecordType,
        brief: String,
        report_link: Option<String>,
    ) -> anyhow::Result<u64> {
        self.modify_task(move |mut task| {
            let id = task.context.next_id();
            if let Some(stage) = task.context.stages.last_mut() {
                stage.records.push(ContextRecord {
                    id,
                    record_type: record_type.clone(),
                    brief,
                    report_link,
                    parent_record_id: None,
                });

                // If this is a report record (Success, Failure, or Comment),
                // reparent all orphaned checkboxes that come before it
                if matches!(record_type, ContextRecordType::Success | ContextRecordType::Failure | ContextRecordType::Comment) {
                    let orphaned_indices: Vec<usize> = stage
                        .records
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, r)| {
                            if idx < stage.records.len() - 1 && // Don't check the newly added record
                                r.parent_record_id.is_none() &&
                                matches!(r.record_type, ContextRecordType::Checkbox(_))
                            {
                                Some(idx)
                            } else {
                                None
                            }
                        })
                        .collect();

                    // Reparent orphaned checkboxes to the new report
                    for idx in orphaned_indices {
                        stage.records[idx].parent_record_id = Some(id);
                    }
                }
            }
            task
        })
        .await?;
        let task = self.get_task().await?;
        let last_id = task
            .context
            .stages
            .last()
            .and_then(|s| s.records.last())
            .map(|r| r.id)
            .unwrap_or(0);
        Ok(last_id)
    }

    /// Record a tool call for transition mapping.
    /// Only `report_success`, `report_failure`, and `report_intermediate` are meaningful transition triggers.
    pub fn record_tool_call(&self, tool: McpTool) {
        match tool {
            McpTool::ReportSuccess | McpTool::ReportFailure | McpTool::ReportIntermediate => {
                *self.last_mapped_tool.lock().unwrap() = Some(tool);
            }
            _ => {}
        }
    }

    /// Get the last MCP tool call that matched a transition key.
    pub fn last_mapped_tool(&self) -> Option<McpTool> {
        *self.last_mapped_tool.lock().unwrap()
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
    pub(crate) fn new(zbobr: Arc<ZbobrDispatcher>, task_id: u64) -> Self {
        Self { zbobr, task_id }
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
        weak.snapshot(false).await
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
    pub async fn set_state(&self, state: impl Into<State>) -> anyhow::Result<()> {
        let state = state.into();
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

    /// Set the pause flag on the task.
    pub async fn set_pause(&self, pause: bool) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.pause = pause;
            task
        })
        .await
    }

    /// Set pause to true and assign a signal in a single modify_task call.
    pub async fn set_pause_with_signal(&self, signal: Signal) -> anyhow::Result<()> {
        self.modify_task(move |mut task| {
            task.pause = true;
            task.signal = Some(signal);
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

    /// Increment the stage counter. Called each time a stage is entered.
    pub async fn increment_stage_count(&self) -> anyhow::Result<()> {
        self.modify_task(move |mut t| {
            t.stage_count += 1;
            t
        })
        .await
    }

    /// Push an entry onto the task's call stack, saving the current pipeline_run_id.
    pub async fn push_stack(
        &self,
        pipeline: impl Into<crate::task::Pipeline>,
        signal: Signal,
    ) -> anyhow::Result<()> {
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

    /// Finish the task: delete placeholder commit, push branch,
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
                match self.zbobr.update_worktree(&identity).await {
                    Ok(true) => {}
                    Ok(false) => {
                        tracing::warn!(
                            "Merge conflict while pushing after placeholder deletion for task #{task_id}"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to push branch after placeholder deletion for task #{task_id}: {e}"
                        );
                    }
                }
            }
        }

        self.modify_task(move |mut task| {
            task.state = State::Done;
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
                None,
            )
            .await
    }
}

#[cfg(test)]
mod comment_model_tests {
    use std::{
        collections::HashMap,
        sync::{Arc, atomic::AtomicU64},
    };

    use async_trait::async_trait;
    use tokio::sync::Mutex;
    use zbobr_api::backend::{TaskBackend, TaskMut, TaskWeak};

    use super::*;
    use crate::{config::ZbobrDispatcherConfig, mcp::traits::CommonMcpImpl};

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

        async fn snapshot(&self, _refresh: bool) -> anyhow::Result<Task> {
            let tasks = self.backend.tasks.lock().await;
            tasks
                .get(&self.id)
                .map(|t| t.task.clone())
                .ok_or_else(|| anyhow::anyhow!("not found"))
        }

        async fn upgrade(&self) -> anyhow::Result<Box<dyn TaskMut>> {
            let lock = self.backend.task_lock(self.id).await;
            let guard = lock.lock_owned().await;
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

    impl TrackingMut {
        async fn mock_store_report(&self, base_name: &str, text: &str) -> String {
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
            filename
        }
    }

    #[async_trait]
    impl TaskMut for TrackingMut {
        fn task_id(&self) -> u64 {
            self.id
        }

        async fn snapshot(&self, _refresh: bool) -> anyhow::Result<Task> {
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

        async fn store_report(&self, base_name: &str, content: &str) -> anyhow::Result<String> {
            Ok(self.mock_store_report(base_name, content).await)
        }

        fn report_url(&self, filename: &str) -> String {
            filename.to_string()
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
            prompt_text: Option<&str>,
        ) -> anyhow::Result<()> {
            let tag = comment_tag(body);
            let report_name = if let Some(text) = report_text {
                let base_name = format!("report_{pipeline}_{pipeline_run_id}_{stage}_{tag}");
                Some(self.mock_store_report(&base_name, text).await)
            } else {
                None
            };

            let prompt_name = if let Some(text) = prompt_text {
                let base_name = format!("prompt_{pipeline}_{pipeline_run_id}_{stage}_{tag}");
                Some(self.mock_store_report(&base_name, text).await)
            } else {
                None
            };

            let mut comments = self.backend.comments.lock().await;
            comments.entry(self.id).or_default().push(Comment {
                timestamp: chrono::Utc::now().with_timezone(chrono::Local::now().offset()),
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
                prompt_name,
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

        async fn list_tasks(&self) -> anyhow::Result<Vec<Box<dyn TaskWeak>>> {
            Ok(vec![])
        }

        async fn create_task(
            &self,
            title: &str,
            description: &str,
            state: zbobr_api::State,
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
                state: state,
                destination_repository: None,
                destination_branch: None,
                work_branch: None,
                pr_url: None,
                context: TaskContext::default(),
                signal: None,
                stack: vec![],
                error: None,
                pause: false,
                confirm: false,
                pipeline_run_id: 0,
                stage_count: 0,
                max_stage_count: 0,
                closed: false,
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

        async fn fetch_refs(&self, _identity: &zbobr_api::TaskIdentity) -> anyhow::Result<()> {
            Ok(())
        }

        async fn ensure_pr_url(&self, _identity: &zbobr_api::TaskIdentity, _body: Option<&str>) -> anyhow::Result<String> {
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
        let zbobr = Arc::new(
            crate::ZbobrDispatcherBuilder::new()
                .with_config(ZbobrDispatcherConfig::default())
                .with_workflow(crate::workflow::Workflow::default())
                .with_task_backend(backend.clone())
                .with_repo_backend(DummyRepo)
                .build(),
        );
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
        let allowed_tools: std::collections::HashSet<zbobr_api::config_tools::McpTool> = [
            zbobr_api::config_tools::McpTool::AddChecklistItem,
            zbobr_api::config_tools::McpTool::StopWithError,
            zbobr_api::config_tools::McpTool::ReportSuccess,
        ]
        .into_iter()
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

        // stop_with_error stores the error in the task's error field
        let _ = planner.stop_with_error_impl("oops").await;

        let weak = task_backend.get_task(id).await.unwrap();
        let task = weak.snapshot(false).await.unwrap();
        assert_eq!(task.error.as_deref(), Some("oops"));
        assert!(task.pause, "stop_with_error should set pause flag");
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
        assert_eq!(
            tag.to_string(),
            "// main:3:planning by host:copilot:gpt-5-mini"
        );
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
        assert_eq!(
            tag_tool_no_model.to_string(),
            "// init:2:working by host:claude"
        );
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
        let tracker = Arc::new(std::sync::Mutex::new(None::<McpTool>));
        let prompt_holder = Arc::new(std::sync::Mutex::new(None::<String>));
        let session = zbobr.role_session_with_tracker(task_id, tracker, "main".to_string(), 1, prompt_holder);
        let allowed_tools: std::collections::HashSet<zbobr_api::config_tools::McpTool> =
            zbobr_api::config_tools::McpTool::all()
                .iter()
                .copied()
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
    async fn report_success_stores_context_records() {
        let (zbobr, task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "desc", "READY", None, None)
            .await
            .unwrap();

        // Add a stage context so records can be attached
        {
            let session = zbobr.role_session(id);
            session
                .modify_task(|mut task| {
                    task.context.stages.push(StageContext {
                        info: StageInfo {
                            pipeline: Pipeline::Main,
                            run_id: 1,
                            stage: Stage::new("working"),
                            tool: Some(Tool::Copilot),
                            model: Some(Model::Gpt5Mini),
                            prompt_link: None,
                            timestamp: "2025-01-01T00:00:00Z".parse().unwrap(),
                        },
                        records: Vec::new(),
                    });
                    task
                })
                .await
                .unwrap();
        }

        let mcp = make_test_mcp(&zbobr, id);

        let _ = mcp
            .report_success_impl("result one", "detailed success report")
            .await;
        let _ = mcp
            .report_failure_impl("needs work", "detailed failure report")
            .await;

        // Reports are stored as context records, not comments
        let weak = task_backend.get_task(id).await.unwrap();
        let task = weak.snapshot(false).await.unwrap();
        let records = &task.context.stages[0].records;
        assert_eq!(records.len(), 2, "two context records should be added");
        assert_eq!(records[0].record_type, ContextRecordType::Success);
        assert_eq!(records[0].brief, "result one");
        assert!(records[0].report_link.is_some());
        assert_eq!(records[1].record_type, ContextRecordType::Failure);
        assert_eq!(records[1].brief, "needs work");
        assert!(records[1].report_link.is_some());

        // Verify reports were stored and are readable
        let success_report = weak
            .read_report(records[0].report_link.as_ref().unwrap())
            .await
            .unwrap();
        assert_eq!(success_report, "detailed success report");
        let failure_report = weak
            .read_report(records[1].report_link.as_ref().unwrap())
            .await
            .unwrap();
        assert_eq!(failure_report, "detailed failure report");
    }

}
