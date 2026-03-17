pub use zbobr_api::task::*;

use std::sync::Arc;

use crate::{
    TaskDir, ZbobrDispatcher,
    backend::{TaskBackend, WorktreeBackend},
};

// ---------------------------------------------------------------------------
// RoleSession — restricted access for MCP tools during agent sessions.
//
// Cannot modify: stage, conflict, confirm (those are dispatcher-only transitions).
// Can modify: description, checklist, parameters, signal, pause.
// ---------------------------------------------------------------------------

/// Restricted task session for MCP tool operations.
/// Stage and conflict flag are protected — only the dispatcher may change them.
#[derive(Clone)]
pub struct RoleSession {
    zbobr: ZbobrDispatcher,
    task_backend: Arc<dyn TaskBackend>,
    task_id: u64,
}

impl RoleSession {
    pub(crate) fn new(zbobr: ZbobrDispatcher, task_backend: Arc<dyn TaskBackend>, task_id: u64) -> Self {
        Self {
            zbobr,
            task_backend,
            task_id,
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
        let weak = self.task_backend.get_task(self.task_id).await?;
        weak.snapshot().await
    }

    /// Get the current task description.
    pub async fn get_description(&self) -> anyhow::Result<String> {
        Ok(self.get_task().await?.description)
    }

    /// Get a history chunk at the given offset.
    /// `offset` is 0-based (0 = oldest chunk); `None` returns the last chunk.
    pub async fn get_history(
        &self,
        offset: Option<usize>,
    ) -> anyhow::Result<zbobr_api::HistoryChunk> {
        crate::get_history(&*self.task_backend, self.task_id, offset).await
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
        let weak = self.task_backend.get_task(self.task_id).await?;
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
        let weak = self.task_backend.get_task(self.task_id).await?;
        weak.get_comments().await
    }

    pub async fn post_comment(
        &self,
        comment_type: CommentType,
        body: &str,
        role: Option<Role>,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
    ) -> anyhow::Result<()> {
        let weak = self.task_backend.get_task(self.task_id).await?;
        let mutable = weak.upgrade().await?;
        mutable
            .post_comment(comment_type, role, hostname, tool, model, body)
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
    zbobr: ZbobrDispatcher,
    task_backend: Arc<dyn TaskBackend>,
    repo_backend: Arc<dyn WorktreeBackend>,
    task_id: u64,
}

impl TaskSession {
    pub(crate) fn new(
        zbobr: ZbobrDispatcher,
        task_backend: Arc<dyn TaskBackend>,
        repo_backend: Arc<dyn WorktreeBackend>,
        task_id: u64,
    ) -> Self {
        Self {
            zbobr,
            task_backend,
            repo_backend,
            task_id,
        }
    }

    pub fn task_id(&self) -> u64 {
        self.task_id
    }

    /// Get a restricted RoleSession view for MCP tool operations.
    pub fn role_session(&self) -> RoleSession {
        RoleSession::new(self.zbobr.clone(), Arc::clone(&self.task_backend), self.task_id)
    }

    /// Read the full task state.
    pub async fn get_task(&self) -> anyhow::Result<Task> {
        let weak = self.task_backend.get_task(self.task_id).await?;
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
        let weak = self.task_backend.get_task(self.task_id).await?;
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
    pub async fn push_stack(&self, mode: &str, stage: &str) -> anyhow::Result<()> {
        let entry = crate::task::StackEntry {
            mode: mode.to_string(),
            stage: stage.to_string(),
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
                if let Err(e) = self.repo_backend.update_pr(&identity).await {
                    tracing::warn!(
                        "Failed to push branch after placeholder deletion for task #{task_id}: {e}"
                    );
                }
            }
        }

        // Post DONE boundary comment.
        let hostname = crate::mcp::common::get_hostname();
        if let Err(e) = self
            .post_comment(CommentType::Done, "", None, &hostname, None, None)
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

    /// Post a structured comment with type, body, and optional role/model metadata.
    pub async fn post_comment(
        &self,
        comment_type: CommentType,
        body: &str,
        role: Option<Role>,
        hostname: &str,
        tool: Option<Tool>,
        model: Option<Model>,
    ) -> anyhow::Result<()> {
        let weak = self.task_backend.get_task(self.task_id).await?;
        let mutable = weak.upgrade().await?;
        mutable
            .post_comment(comment_type, role, hostname, tool, model, body)
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
            comment_type: CommentType,
            role: Option<Role>,
            hostname: &str,
            tool: Option<Tool>,
            model: Option<Model>,
            body: &str,
        ) -> anyhow::Result<()> {
            let mut comments = self.backend.comments.lock().await;
            comments.entry(self.id).or_default().push(Comment {
                comment_type,
                timestamp: String::new(),
                role,
                hostname: hostname.to_string(),
                tool,
                model,
                text: body.to_string(),
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

    fn make_test_parts() -> (crate::ZbobrDispatcher, Arc<ArcTrackingBackend>, Arc<DummyRepo>) {
        let backend = Arc::new(ArcTrackingBackend {
            inner: Arc::new(TrackingBackend {
                tasks: Mutex::new(HashMap::new()),
                comments: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(0),
                locks: Mutex::new(HashMap::new()),
            }),
        });
        let repo = Arc::new(DummyRepo);
        let zbobr = crate::ZbobrDispatcherBuilder::new()
            .with_config(Arc::new(ZbobrDispatcherConfig::default()))
            .with_task_backend(backend.clone() as Arc<dyn TaskBackend>)
            .with_repo_backend(repo.clone() as Arc<dyn WorktreeBackend>)
            .build();
        (zbobr, backend, repo)
    }

    #[tokio::test]
    async fn mcp_helper_includes_explicit_model() {
        let (zbobr, task_backend, _repo_backend) = make_test_parts();
        let id = zbobr
            .create_task(&*task_backend, "t", "", "READY", None, None)
            .await
            .unwrap();

        let planner = crate::mcp::planner::PlannerMcp::new(
            zbobr.clone(),
            task_backend.clone() as Arc<dyn TaskBackend>,
            id,
            Tool::Copilot,
            Model::Gpt5Mini,
            "planning".to_string(),
        );

        let _ = planner.report_error_impl("oops").await;

        let weak = task_backend.get_task(id).await.unwrap();
        let comments = weak.get_comments().await.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].model, Some(Model::Gpt5Mini));
    }

    #[tokio::test]
    async fn dispatcher_posts_have_no_model() {
        let (zbobr, task_backend, repo_backend) = make_test_parts();
        let id = zbobr
            .create_task(&*task_backend, "t", "", "READY", None, None)
            .await
            .unwrap();

        zbobr
            .task_session(task_backend.clone() as Arc<dyn TaskBackend>, repo_backend.clone() as Arc<dyn WorktreeBackend>, id)
            .post_comment(
                CommentType::Error,
                "dispatcher error",
                None::<Role>,
                "host",
                None::<Tool>,
                None::<Model>,
            )
            .await
            .unwrap();

        let weak = task_backend.get_task(id).await.unwrap();
        let comments = weak.get_comments().await.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].model, None);
    }

    #[test]
    fn comment_tag_roundtrip_models() {
        let tag = CommentTag::new(
            CommentType::Request,
            Some(Role::Planner),
            "host".to_string(),
            None,
            Some(Model::Gpt5Mini),
        );
        assert_eq!(tag.to_string(), "// REQUEST planner:host:gpt-5-mini");
        let parsed: CommentTag = tag.to_string().parse().unwrap();
        assert_eq!(parsed, tag);

        let tag_user = CommentTag::new(CommentType::Request, None, "web".to_string(), None, None);
        assert_eq!(tag_user.to_string(), "// REQUEST user:web");

        let tag_default = CommentTag::new(
            CommentType::Report,
            Some(Role::Planner),
            "host".to_string(),
            Some(Tool::Copilot),
            Some(Model::Default),
        );
        assert_eq!(
            tag_default.to_string(),
            "// REPORT planner:host:copilot:default"
        );
        let parsed_default: CommentTag = tag_default.to_string().parse().unwrap();
        assert_eq!(parsed_default, tag_default);

        let tag_tool = CommentTag::new(
            CommentType::Report,
            Some(Role::Worker),
            "host".to_string(),
            Some(Tool::Copilot),
            Some(Model::Gpt5Mini),
        );
        assert_eq!(
            tag_tool.to_string(),
            "// REPORT worker:host:copilot:gpt-5-mini"
        );
        let parsed_tool: CommentTag = tag_tool.to_string().parse().unwrap();
        assert_eq!(parsed_tool, tag_tool);
    }
}
