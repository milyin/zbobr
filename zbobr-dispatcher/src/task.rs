pub use zbobr_api::task::*;

use std::sync::Arc;

use crate::{
    TaskDir, ZbobrDispatcher,
};

// ---------------------------------------------------------------------------
// Comment buffering
// ---------------------------------------------------------------------------

/// A comment buffered during an MCP stage, to be flushed as a single combined
/// comment at the end of the stage. Each entry's `body` is already prefixed
/// with `[tool_name]\n` by the MCP tool that posted it.
#[derive(Debug, Clone)]
pub struct BufferedComment {
    pub body: String,
}

/// Shared comment buffer used to group per-stage MCP comments.
pub type CommentBuffer = Arc<std::sync::Mutex<Vec<BufferedComment>>>;

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
    /// When present, `post_comment` appends to this buffer instead of posting
    /// directly. The buffer is flushed as a single combined comment at stage end.
    comment_buffer: Option<CommentBuffer>,
}

impl RoleSession {
    pub(crate) fn new(zbobr: Arc<ZbobrDispatcher>, task_id: u64) -> Self {
        Self {
            zbobr,
            task_id,
            last_mapped_tool: Arc::new(std::sync::Mutex::new(None)),
            comment_buffer: None,
        }
    }

    pub(crate) fn with_shared_tracker(
        zbobr: Arc<ZbobrDispatcher>,
        task_id: u64,
        tracker: Arc<std::sync::Mutex<Option<String>>>,
        comment_buffer: CommentBuffer,
    ) -> Self {
        Self {
            zbobr,
            task_id,
            last_mapped_tool: tracker,
            comment_buffer: Some(comment_buffer),
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

    /// Build the full comment list including buffered comments.
    async fn comments_with_buffer(&self) -> anyhow::Result<(Vec<Comment>, String)> {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        let mut comments = weak.get_comments().await?;
        let task = weak.snapshot().await?;
        if let Some(ref buffer) = self.comment_buffer {
            let buffered = buffer.lock().unwrap();
            for bc in buffered.iter() {
                comments.push(Comment {
                    timestamp: String::new(),
                    stage: String::new(),
                    hostname: String::new(),
                    tool: None,
                    model: None,
                    text: bc.body.clone(),
                    hidden: false,
                });
            }
        }
        Ok((comments, task.description))
    }

    /// Get the history index (all records with position, author, type, summary).
    pub async fn get_history_index(&self) -> anyhow::Result<zbobr_api::HistoryIndex> {
        let (comments, description) = self.comments_with_buffer().await?;
        Ok(zbobr_api::build_history_index(&comments, &description))
    }

    /// Get a single history record by position index.
    pub async fn get_history_record(&self, index: usize) -> anyhow::Result<String> {
        let (comments, description) = self.comments_with_buffer().await?;
        zbobr_api::get_history_record_by_index(&comments, &description, index)
    }

    /// Get the current task checklist.
    pub async fn get_checklist(&self) -> anyhow::Result<Vec<ChecklistItem>> {
        Ok(self.get_task().await?.checklist)
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

    /// Post a comment. When `buffered` is true and a comment buffer is active,
    /// the comment is accumulated and will be flushed at stage end. When false
    /// (or no buffer), the comment is posted immediately to the backend.
    pub async fn post_comment(
        &self,
        body: &str,
        stage: &str,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
        buffered: bool,
        hidden: bool,
    ) -> anyhow::Result<()> {
        if buffered {
            if let Some(ref buffer) = self.comment_buffer {
                buffer.lock().unwrap().push(BufferedComment {
                    body: body.to_string(),
                });
                return Ok(());
            }
        }
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        let mutable = weak.upgrade().await?;
        mutable
            .post_comment(stage, hostname, tool, model, body, hidden)
            .await
    }

    /// Get the current signal on the task.
    pub async fn get_signal(&self) -> anyhow::Result<Option<String>> {
        let task = self.get_task().await?;
        Ok(task.signal)
    }

    /// Set signal on the task.
    pub async fn set_signal(&self, new_signal: &str) -> anyhow::Result<()> {
        let signal = new_signal.to_string();
        self.modify_task(move |mut task| {
            task.signal = Some(signal);
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
    pub async fn set_signal(&self, signal: Option<&str>) -> anyhow::Result<()> {
        let signal = signal.map(|s| s.to_string());
        self.modify_task(move |mut task| {
            task.signal = signal;
            task
        })
        .await
    }

    /// Push an entry onto the task's call stack.
    pub async fn push_stack(&self, pipeline: &str, signal: &str) -> anyhow::Result<()> {
        let entry = crate::task::StackEntry {
            pipeline: pipeline.to_string(),
            signal: signal.to_string(),
        };
        self.modify_task(move |mut task| {
            task.stack.push(entry);
            task
        })
        .await
    }

    /// Pop the top entry from the task's call stack.
    pub async fn pop_stack(&self) -> anyhow::Result<Option<crate::task::StackEntry>> {
        let task = self.get_task().await?;
        let popped = task.stack.last().cloned();
        if popped.is_some() {
            self.modify_task(move |mut task| {
                task.stack.pop();
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

        // Post DONE marker comment (hidden).
        let hostname = crate::mcp::common::get_hostname();
        if let Err(e) = self
            .post_comment("done", &hostname, None, None, "", true)
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
        hidden: bool,
    ) -> anyhow::Result<()> {
        let weak = self.zbobr.task_backend().get_task(self.task_id).await?;
        let mutable = weak.upgrade().await?;
        mutable
            .post_comment(stage, hostname, tool, model, body, hidden)
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
            hidden: bool,
        ) -> anyhow::Result<()> {
            let mut comments = self.backend.comments.lock().await;
            comments.entry(self.id).or_default().push(Comment {
                timestamp: String::new(),
                stage: stage.to_string(),
                hostname: hostname.to_string(),
                tool,
                model,
                text: body.to_string(),
                hidden,
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
                worktree_retries: 0,
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
            ["get_history_index", "stop_with_error", "report_success"]
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
        );

        // stop_with_error is unbuffered — goes straight to backend
        let _ = planner.stop_with_error_impl("oops").await;

        let weak = task_backend.get_task(id).await.unwrap();
        let comments = weak.get_comments().await.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].model, Some(Model::Gpt5Mini));
        assert_eq!(comments[0].tool, Some(Tool::Copilot));
        assert!(comments[0].hidden);
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
            .post_comment("error", "host", None, None, "dispatcher error", true)
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
            "planning".to_string(),
            "host".to_string(),
            Some(Tool::Copilot),
            Some(Model::Gpt5Mini),
            false,
        );
        assert_eq!(tag.to_string(), "// planning:host:copilot:gpt-5-mini");
        let parsed: CommentTag = tag.to_string().parse().unwrap();
        assert_eq!(parsed, tag);

        let tag_no_tool = CommentTag::new(
            "done".to_string(),
            "web".to_string(),
            None,
            None,
            true,
        );
        assert_eq!(tag_no_tool.to_string(), "// done:web:hidden");
        let parsed_no_tool: CommentTag = tag_no_tool.to_string().parse().unwrap();
        assert_eq!(parsed_no_tool, tag_no_tool);

        let tag_tool_no_model = CommentTag::new(
            "working".to_string(),
            "host".to_string(),
            Some(Tool::Claude),
            None,
            false,
        );
        assert_eq!(tag_tool_no_model.to_string(), "// working:host:claude");
        let parsed_tnm: CommentTag = tag_tool_no_model.to_string().parse().unwrap();
        assert_eq!(parsed_tnm, tag_tool_no_model);
    }

    // -----------------------------------------------------------------------
    // Helper: create a UnifiedMcp with comment buffering enabled
    // -----------------------------------------------------------------------

    fn make_buffered_mcp(
        zbobr: &Arc<crate::ZbobrDispatcher>,
        task_id: u64,
    ) -> (crate::mcp::unified::UnifiedMcp, CommentBuffer) {
        let tracker = Arc::new(std::sync::Mutex::new(None::<String>));
        let comment_buffer: CommentBuffer = Arc::new(std::sync::Mutex::new(Vec::new()));
        let session = zbobr.role_session_with_tracker(
            task_id,
            tracker,
            Arc::clone(&comment_buffer),
        );
        let allowed_tools: std::collections::HashSet<String> = crate::mcp::unified::ALL_TOOL_NAMES
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mcp = crate::mcp::unified::UnifiedMcp::new(
            session,
            allowed_tools,
            "worker".to_string(),
            Tool::Copilot,
            Model::Gpt5Mini,
            "working".to_string(),
        );
        (mcp, comment_buffer)
    }

    // -----------------------------------------------------------------------
    // Buffering tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn buffered_comments_accumulate_in_buffer_not_backend() {
        let (zbobr, task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "desc", "READY", None, None)
            .await
            .unwrap();

        let (mcp, comment_buffer) =
            make_buffered_mcp(&zbobr, id);

        // report_success is buffered
        let _ = mcp.report_success_impl("result one").await;
        let _ = mcp.report_failure_impl("needs work").await;

        // Backend should have zero comments (all buffered)
        let weak = task_backend.get_task(id).await.unwrap();
        let backend_comments = weak.get_comments().await.unwrap();
        assert_eq!(backend_comments.len(), 0, "buffered comments must not reach backend");

        // Buffer should have two entries
        let buf = comment_buffer.lock().unwrap();
        assert_eq!(buf.len(), 2);
    }

    #[tokio::test]
    async fn error_comments_are_unbuffered() {
        let (zbobr, task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "desc", "READY", None, None)
            .await
            .unwrap();

        let (mcp, comment_buffer) =
            make_buffered_mcp(&zbobr, id);

        // stop_with_error is unbuffered — should go directly to backend
        let _ = mcp.stop_with_error_impl("something broke").await;

        // Backend should have the error comment
        let weak = task_backend.get_task(id).await.unwrap();
        let backend_comments = weak.get_comments().await.unwrap();
        assert_eq!(backend_comments.len(), 1, "error must be posted to backend immediately");
        assert!(backend_comments[0].hidden, "error comments must be hidden");
        assert!(
            backend_comments[0].text.starts_with("[stop_with_error]"),
            "error comment must be prefixed with [stop_with_error]"
        );

        // Buffer should be empty
        let buf = comment_buffer.lock().unwrap();
        assert_eq!(buf.len(), 0, "error must not be buffered");
    }

    #[tokio::test]
    async fn stop_with_question_is_unbuffered() {
        let (zbobr, task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "desc", "READY", None, None)
            .await
            .unwrap();

        let (mcp, comment_buffer) =
            make_buffered_mcp(&zbobr, id);

        let _ = mcp.stop_with_question_impl("need help").await;

        // stop_with_question is unbuffered — posted directly
        let weak = task_backend.get_task(id).await.unwrap();
        let backend_comments = weak.get_comments().await.unwrap();
        assert_eq!(backend_comments.len(), 1, "stop_with_question must be posted to backend immediately");
        assert!(
            backend_comments[0].text.starts_with("[stop_with_question]"),
            "stop_with_question comment must be prefixed with [stop_with_question]"
        );

        let buf = comment_buffer.lock().unwrap();
        assert_eq!(buf.len(), 0, "stop_with_question must not be buffered");
    }

    #[tokio::test]
    async fn each_buffered_comment_marked_by_tool_name() {
        let (zbobr, _task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "desc", "READY", None, None)
            .await
            .unwrap();

        let (mcp, comment_buffer) =
            make_buffered_mcp(&zbobr, id);

        // Call the two buffered MCP tools
        let _ = mcp.report_success_impl("first results").await;
        let _ = mcp.report_failure_impl("needs fixes").await;

        let buf = comment_buffer.lock().unwrap();
        assert_eq!(buf.len(), 2);

        // Each buffered comment body starts with [tool_name]
        assert!(buf[0].body.starts_with("[report_success]\n"), "got: {}", buf[0].body);
        assert!(buf[1].body.starts_with("[report_failure]\n"), "got: {}", buf[1].body);
    }

    #[tokio::test]
    async fn buffered_comments_visible_in_get_history_index() {
        let (zbobr, _task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "task description", "READY", None, None)
            .await
            .unwrap();

        let (mcp, _comment_buffer) =
            make_buffered_mcp(&zbobr, id);

        // Post a buffered comment
        let _ = mcp.report_success_impl("my results").await;

        // get_history_index should include the buffered comment
        let index = mcp.get_history_index_impl().await;
        assert!(
            index.contains("report_success") || index.contains("success"),
            "buffered comment must be visible in get_history_index response"
        );
    }

    #[tokio::test]
    async fn mixed_buffered_and_unbuffered_ordering() {
        let (zbobr, task_backend) = make_test_parts();
        let id = zbobr
            .create_task("t", "desc", "READY", None, None)
            .await
            .unwrap();

        let (mcp, comment_buffer) =
            make_buffered_mcp(&zbobr, id);

        // Buffered
        let _ = mcp.report_success_impl("first").await;
        // Unbuffered (error)
        let _ = mcp.stop_with_error_impl("oops").await;
        // Buffered
        let _ = mcp.report_failure_impl("plan").await;

        // Backend should have exactly 1 comment (the error)
        let weak = task_backend.get_task(id).await.unwrap();
        let backend_comments = weak.get_comments().await.unwrap();
        assert_eq!(backend_comments.len(), 1);
        assert!(backend_comments[0].text.starts_with("[stop_with_error]"));

        // Buffer should have 2 entries
        let buf = comment_buffer.lock().unwrap();
        assert_eq!(buf.len(), 2);
        assert!(buf[0].body.starts_with("[report_success]"));
        assert!(buf[1].body.starts_with("[report_failure]"));
    }
}
