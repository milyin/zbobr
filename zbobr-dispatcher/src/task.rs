pub use zbobr_api::task::*;

use crate::ZbobrDispatcherDyn;

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
    zbobr: ZbobrDispatcherDyn,
    task_id: u64,
}

impl RoleSession {
    pub(crate) fn new(zbobr: ZbobrDispatcherDyn, task_id: u64) -> Self {
        Self { zbobr, task_id }
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
        self.zbobr.get_task(self.task_id).await
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
        self.zbobr.get_history(self.task_id, offset).await
    }

    /// Get the current task checklist.
    pub async fn get_checklist(&self) -> anyhow::Result<Vec<ChecklistItem>> {
        Ok(self.get_task().await?.checklist)
    }

    /// Atomically read-modify-write the task body.
    ///
    /// The closure receives a mutable `Task` reference and may modify `description`,
    /// `parameters`, `checklist`, `signal`, and `pause`.
    ///
    /// **Protected fields**: `stage` and `conflict` are saved before the mutation
    /// and restored afterwards, so MCP tools cannot change them.
    pub async fn modify_task<F>(&self, mutate: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Task) + Send + 'static,
    {
        let lock = self.zbobr.task_lock(self.task_id);
        let _guard = lock.lock().await;

        self.zbobr
            .task_backend
            .modify_task(
                self.task_id,
                Box::new(move |mut task| {
                    let saved_stage = task.stage;
                    let saved_conflict = task.conflict;
                    mutate(&mut task);
                    task.stage = saved_stage;
                    task.conflict = saved_conflict;
                    task
                }),
            )
            .await
    }

    /// Get all comments as structured `Comment` objects (includes all
    /// types: error, report, request, plan, etc.).
    pub async fn get_comments(&self) -> anyhow::Result<Vec<Comment>> {
        self.zbobr.get_task_comments(self.task_id).await
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
        // dispatcher/session API simply forwards whatever tool/model the caller
        // provides.  MCP helpers will pass the concrete values; dispatcher-only
        // code uses `None` for both.
        self.zbobr
            .post_task_comment(
                self.task_id,
                comment_type,
                role,
                hostname,
                tool,
                model,
                body,
            )
            .await
    }

    /// Get the current signal on the task.
    pub async fn get_signal(&self) -> anyhow::Result<Option<Signal>> {
        let task = self.zbobr.get_task(self.task_id).await?;
        Ok(task.signal)
    }

    /// Set signal on the task, respecting priority (higher priority signals cannot be overwritten by lower).
    pub async fn set_signal(&self, new_signal: Signal) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            // Only set if new signal has higher or equal priority (lower enum value)
            if let Some(current_signal) = task.signal
                && new_signal > current_signal
            {
                // new_signal has lower priority, don't overwrite
                return;
            }
            task.signal = Some(new_signal);
        })
        .await
    }

    /// Clear the signal on the task.
    pub async fn clear_signal(&self) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            task.signal = None;
        })
        .await
    }

    /// Set the pause flag on the task.
    pub async fn set_pause(&self, pause: bool) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            task.pause = pause;
        })
        .await
    }

    /// Ensure `pr_url` is stored in task parameters.
    ///
    /// If already set, returns the existing value immediately.
    /// If not set: calls `update_pr` on the backend, stores the result in
    /// `Parameter::PrUrl`, and returns it.
    pub async fn ensure_pr_url(&self) -> anyhow::Result<String> {
        let task = self.get_task().await?;
        if let Some(url) = task.parameters.get(&Parameter::PrUrl).cloned() {
            return Ok(url);
        }
        let pr_url = self.update_pr().await?;
        self.set_parameter(Parameter::PrUrl, Some(pr_url.clone()))
            .await?;
        Ok(pr_url)
    }

    /// Push current work branch commits to the remote by updating PR state.
    ///
    /// Reads `WorkBranch` from task parameters and calls `update_pr` on the backend
    /// to sync the remote state. Result URL (if any) is discarded.
    pub async fn update_pr(&self) -> anyhow::Result<String> {
        let task = self.get_task().await?;
        let work_branch = task
            .parameters
            .get(&Parameter::WorkBranch)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("work_branch parameter is not set"))?;
        self.zbobr.update_pr(&work_branch).await
    }

    /// Get a task parameter value. Parameters are stored in the task's parameters HashMap.
    pub async fn get_parameter(&self, param: Parameter) -> anyhow::Result<Option<String>> {
        let task = self.zbobr.get_task(self.task_id).await?;
        Ok(task.parameters.get(&param).cloned())
    }

    /// Set a task parameter value with automatic conflict detection.
    /// Parameters are stored in the visible PARAMETERS section.
    pub async fn set_parameter(
        &self,
        param: Parameter,
        value: Option<String>,
    ) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            if let Some(v) = value {
                task.parameters.insert(param, v);
            } else {
                task.parameters.remove(&param);
            }
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
    zbobr: ZbobrDispatcherDyn,
    task_id: u64,
}

impl TaskSession {
    pub(crate) fn new(zbobr: ZbobrDispatcherDyn, task_id: u64) -> Self {
        Self { zbobr, task_id }
    }

    pub fn task_id(&self) -> u64 {
        self.task_id
    }

    /// Get a restricted RoleSession view for MCP tool operations.
    pub fn role_session(&self) -> RoleSession {
        RoleSession::new(self.zbobr.clone(), self.task_id)
    }

    /// Read the full task state.
    pub async fn get_task(&self) -> anyhow::Result<Task> {
        self.zbobr.get_task(self.task_id).await
    }

    /// Get the current task checklist.
    pub async fn get_checklist(&self) -> anyhow::Result<Vec<ChecklistItem>> {
        Ok(self.get_task().await?.checklist)
    }

    /// Atomically read-modify-write the task with unrestricted access.
    /// Only the dispatcher should use this.
    pub async fn modify_task<F>(&self, mutate: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Task) + Send + 'static,
    {
        let lock = self.zbobr.task_lock(self.task_id);
        let _guard = lock.lock().await;

        self.zbobr
            .task_backend
            .modify_task(
                self.task_id,
                Box::new(move |mut task| {
                    mutate(&mut task);
                    task
                }),
            )
            .await
    }

    /// Set the task stage (dispatcher only).
    pub async fn set_stage(&self, stage: Stage) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            // if confirm flag is enabled we always pause on any stage transition
            if task.confirm && task.stage != stage {
                task.pause = true;
            }
            task.stage = stage;
        })
        .await
    }

    /// Set the confirm flag on the task (dispatcher only).
    /// This is convenience used by create_task_with_confirm.
    pub async fn set_confirm(&self, confirm: bool) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            task.confirm = confirm;
        })
        .await
    }

    /// Set the conflict flag (dispatcher only).
    pub async fn set_conflict(&self, conflict: bool) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            task.conflict = conflict;
        })
        .await
    }

    /// Set signal on the task (dispatcher only, no priority check).
    pub async fn set_signal(&self, signal: Option<Signal>) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            task.signal = signal;
        })
        .await
    }

    /// Finish the task: delete placeholder commit, push branch, post Done comment,
    /// then set stage to Done and clear signal.
    pub async fn finish(&self) -> anyhow::Result<()> {
        let task_id = self.task_id;
        let task = self.get_task().await?;

        // Delete placeholder commit and push before marking done.
        if let Some(work_branch) = task.parameters.get(&Parameter::WorkBranch).cloned() {
            let task_dir = self
                .zbobr
                .config()
                .workspaces
                .join(format!("task#{task_id}"));
            let work_dir =
                if let Some(dest_repo) = task.parameters.get(&Parameter::DestinationRepository) {
                    let repo_name = dest_repo.rsplit('/').next().unwrap_or(dest_repo.as_str());
                    task_dir.join(repo_name)
                } else {
                    task_dir
                };
            if let Err(e) = zbobr_utility::delete_placeholder_commit(&work_dir, &work_branch).await
            {
                tracing::warn!("Failed to delete placeholder commit for task #{task_id}: {e}");
            } else {
                let role_session = self.role_session();
                if let Err(e) = role_session.update_pr().await {
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

        self.modify_task(move |task| {
            task.stage = Stage::Done;
            task.signal = None;
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
        // simply forward provided metadata; dispatcher-level callers send
        // `None` for both tool and model so tags remain minimal.

        self.zbobr
            .post_task_comment(
                self.task_id,
                comment_type,
                role,
                hostname,
                tool,
                model,
                body,
            )
            .await
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ZbobrDispatcherConfig;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::sync::Mutex;

    #[test]
    fn stage_milestone_names() {
        assert_eq!(Stage::Pending.milestone_name(), "PENDING");
        assert_eq!(Stage::Planning.milestone_name(), "PLANNING");
        assert_eq!(Stage::Working.milestone_name(), "WORKING");
        assert_eq!(Stage::Reviewing.milestone_name(), "REVIEWING");
        assert_eq!(Stage::Preparing.milestone_name(), "PREPARING");
        assert_eq!(Stage::Merging.milestone_name(), "MERGING");
        assert_eq!(Stage::Done.milestone_name(), "DONE");
    }

    #[test]
    fn stage_backward_compat() {
        assert_eq!(
            Stage::from_milestone_name("PREPARATION"),
            Some(Stage::Preparing)
        );
        assert_eq!(
            Stage::from_milestone_name("PREPARING"),
            Some(Stage::Preparing)
        );
    }

    #[test]
    fn stage_display() {
        assert_eq!(Stage::Planning.to_string(), "PLANNING");
        assert_eq!(Stage::Working.to_string(), "WORKING");
        assert_eq!(Stage::Reviewing.to_string(), "REVIEWING");
        assert_eq!(Stage::Done.to_string(), "DONE");
    }

    #[test]
    fn convert_role_to_stage() {
        assert_eq!(Stage::from(Role::Preparator), Stage::Preparing);
        assert_eq!(Stage::from(Role::Planner), Stage::Planning);
        assert_eq!(Stage::from(Role::Worker), Stage::Working);
        assert_eq!(Stage::from(Role::Reviewer), Stage::Reviewing);
        assert_eq!(Stage::from(Role::Merger), Stage::Merging);
        // user is mapped to a neutral stage so conversions remain total
        assert_eq!(Stage::from(Role::User), Stage::Pending);
    }

    #[test]
    fn comment_tag_roundtrip() {
        let tag = CommentTag::new(
            CommentType::Request,
            Role::Planner,
            "host".to_string(),
            None,
            Some(Model::Claude3Opus),
        );
        assert_eq!(tag.to_string(), "// REQUEST planner:host:claude-opus");
        let parsed: CommentTag = tag.to_string().parse().unwrap();
        assert_eq!(parsed, tag);

        let tag_user = CommentTag::new(
            CommentType::Request,
            Role::User,
            "web".to_string(),
            None,
            None,
        );
        assert_eq!(tag_user.to_string(), "// REQUEST user:web");
    }

    #[test]
    fn try_convert_stage_to_role() {
        assert_eq!(Role::try_from(Stage::Preparing).unwrap(), Role::Preparator);
        assert_eq!(Role::try_from(Stage::Planning).unwrap(), Role::Planner);
        assert_eq!(Role::try_from(Stage::Working).unwrap(), Role::Worker);
        assert_eq!(Role::try_from(Stage::Reviewing).unwrap(), Role::Reviewer);
        assert_eq!(Role::try_from(Stage::Merging).unwrap(), Role::Merger);
        assert!(Role::try_from(Stage::Pending).is_err());
        assert!(Role::try_from(Stage::Done).is_err());
    }

    #[test]
    fn stage_roundtrip_serde() {
        let stage = Stage::Planning;
        let json = serde_json::to_string(&stage).unwrap();
        let back: Stage = serde_json::from_str(&json).unwrap();
        assert_eq!(back, stage);
    }

    #[test]
    fn role_serde_user() {
        let role = Role::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");
        let back: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(back, role);
    }

    #[test]
    fn task_serde() {
        let task = Task {
            id: 42,
            title: "Test task".to_string(),
            description: "Do something".to_string(),
            stage: Stage::Planning,
            tool: Some(Tool::Claude),
            model: Some(Model::Claude3Opus),
            parameters: HashMap::new(),
            checklist: vec![],
            signal: None,
            conflict: false,
            pause: false,
            confirm: false,
            etag: None,
        };
        let json = serde_json::to_string(&task).unwrap();
        let back: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 42);
        assert_eq!(back.title, "Test task");
        assert_eq!(back.stage, Stage::Planning);
        assert_eq!(back.tool, Some(Tool::Claude));
        assert_eq!(back.model, Some(Model::Claude3Opus));
        assert!(!back.conflict);
        assert!(!back.pause);
        assert!(!back.confirm);
    }

    #[test]
    fn confirm_flag_triggers_pause_on_stage_change() {
        let mut t = Task {
            id: 1,
            title: "x".into(),
            description: "".into(),
            stage: Stage::Pending,
            tool: None,
            model: None,
            parameters: HashMap::new(),
            checklist: vec![],
            signal: None,
            conflict: false,
            pause: false,
            confirm: true,
            etag: None,
        };
        let new_stage = Stage::Planning;
        if t.confirm && t.stage != new_stage {
            t.pause = true;
        }
        t.stage = new_stage;
        assert!(
            t.pause,
            "task should have been paused when confirm=true and stage changed"
        );
    }

    #[test]
    fn update_closure_applies_confirm_first() {
        // simulate the modify_task closure used by the CLI update command
        let mut task = Task {
            id: 2,
            title: "x".into(),
            description: "".into(),
            stage: Stage::Pending,
            tool: None,
            model: None,
            parameters: HashMap::new(),
            checklist: vec![],
            signal: None,
            conflict: false,
            pause: false,
            confirm: false,
            etag: None,
        };
        let new_confirm = Some(true);
        let new_stage = Some(Stage::Planning);

        if let Some(c) = new_confirm {
            task.confirm = c;
        }
        if let Some(s) = new_stage {
            if task.confirm && task.stage != s {
                task.pause = true;
            }
            task.stage = s;
        }

        assert!(
            task.pause,
            "pause must be set when confirm is enabled and stage changes"
        );
    }

    // --- Shared async test infrastructure ---

    struct DummyBackend {
        tasks: Mutex<HashMap<u64, Task>>,
        next_id: AtomicU64,
    }

    #[async_trait]
    impl crate::backend::TaskBackend for DummyBackend {
        async fn get_task(&self, id: u64) -> anyhow::Result<Task> {
            let tasks = self.tasks.lock().await;
            tasks
                .get(&id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("not found"))
        }
        async fn create_task(
            &self,
            title: &str,
            description: &str,
            stage: Stage,
            tool: Option<Tool>,
            model: Option<Model>,
            parameters: HashMap<Parameter, String>,
        ) -> anyhow::Result<u64> {
            let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
            let task = Task {
                id,
                title: title.to_string(),
                description: description.to_string(),
                stage,
                tool,
                model,
                parameters,
                checklist: vec![],
                signal: None,
                conflict: false,
                pause: false,
                confirm: false,
                etag: None,
            };
            self.tasks.lock().await.insert(id, task);
            Ok(id)
        }
        async fn close_task(&self, _id: u64) -> anyhow::Result<()> {
            Ok(())
        }
        async fn is_task_closed(&self, _id: u64) -> anyhow::Result<bool> {
            Ok(false)
        }
        async fn modify_task(
            &self,
            id: u64,
            mutate: Box<dyn FnOnce(Task) -> Task + Send>,
        ) -> anyhow::Result<()> {
            let mut tasks = self.tasks.lock().await;
            if let Some(t) = tasks.remove(&id) {
                let t = mutate(t);
                tasks.insert(id, t);
                Ok(())
            } else {
                Err(anyhow::anyhow!("not found"))
            }
        }
        async fn list_tasks_by_stage(
            &self,
            _stage: Stage,
            _tool: Option<Tool>,
        ) -> anyhow::Result<Vec<Task>> {
            Ok(vec![])
        }
        async fn get_task_comments(&self, _id: u64) -> anyhow::Result<Vec<String>> {
            Ok(vec![])
        }
        async fn post_task_comment(
            &self,
            _id: u64,
            _body: &str,
            _role: Role,
            _hostname: &str,
            _tool: Option<Tool>,
            _model: Option<Model>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn setup(&self, _force: bool) -> anyhow::Result<()> {
            Ok(())
        }
        async fn validate_connectivity(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn debug_state(&self) -> String {
            "dummy".to_string()
        }
    }

    struct DummyRepo;
    #[async_trait]
    impl crate::backend::WorktreeBackend for DummyRepo {
        async fn update_worktree(
            &self,
            _remote_repo: &str,
            _base_branch: &str,
            _work_branch: &str,
            _workspace_path: &std::path::Path,
        ) -> anyhow::Result<bool> {
            Ok(true)
        }

        async fn update_pr(&self, _work_branch: &str) -> anyhow::Result<String> {
            Ok("mock-pr-url".to_string())
        }

        async fn validate_connectivity(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn debug_state(&self) -> String {
            "dummy".to_string()
        }
    }

    fn make_test_zbobr() -> crate::ZbobrDispatcherDyn {
        let backend: Arc<dyn crate::backend::TaskBackend> = Arc::new(DummyBackend {
            tasks: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(0),
        });
        let repo: Arc<dyn crate::backend::WorktreeBackend> = Arc::new(DummyRepo);
        crate::ZbobrDispatcher::new_with_backends(ZbobrDispatcherConfig::default(), backend, repo)
    }

    // asynchronous tests require a runtime; use tokio for the following
    #[tokio::test]
    async fn task_session_set_stage_pauses_when_confirm() {
        let zbobr = make_test_zbobr();
        let id = zbobr
            .create_task("t", "", Stage::Pending, None, None, None, None)
            .await
            .unwrap();
        zbobr.task_session(id).set_confirm(true).await.unwrap();
        zbobr
            .task_session(id)
            .set_stage(Stage::Planning)
            .await
            .unwrap();
        let t = zbobr.get_task(id).await.unwrap();
        assert!(t.pause);
    }

    #[tokio::test]
    async fn set_pause_preserves_signal() {
        // Verify set_pause(true) does not clear the signal — this is the contract
        // that report_error and ask_user (which call set_pause) must uphold so the
        // dispatcher can still route the task after the user responds.
        let zbobr = make_test_zbobr();
        let id = zbobr
            .create_task("t", "", Stage::Working, None, None, None, None)
            .await
            .unwrap();
        // Use role_session which has the same set_signal/set_pause methods as MCP commands
        zbobr
            .role_session(id)
            .set_signal(Signal::GoWork)
            .await
            .unwrap();
        zbobr.role_session(id).set_pause(true).await.unwrap();
        let t = zbobr.get_task(id).await.unwrap();
        assert_eq!(
            t.signal,
            Some(Signal::GoWork),
            "signal must not be cleared by set_pause"
        );
        assert!(t.pause);
    }

    #[test]
    fn signal_target_role() {
        assert_eq!(Signal::GoPrepare.target_role(), Role::Preparator);
        assert_eq!(Signal::GoPlan.target_role(), Role::Planner);
        assert_eq!(Signal::GoWork.target_role(), Role::Worker);
        assert_eq!(Signal::GoReview.target_role(), Role::Reviewer);
    }

    #[test]
    fn model_mapping() {
        // specific mappings
        assert_eq!(
            Model::Gpt4o.model_name_for_tool(Tool::Copilot),
            Some("gpt-4o")
        );
        assert_eq!(
            Model::ClaudeSonnet4_5.model_name_for_tool(Tool::Copilot),
            Some("claude-sonnet-4.5")
        );
        assert_eq!(
            Model::Claude35Sonnet.model_name_for_tool(Tool::Claude),
            Some("claude-3-5-sonnet")
        );
        assert_eq!(Model::Gpt5_2.model_name_for_tool(Tool::Claude), None);

        // the "default" sentinel should produce the cheapest supported
        // option for each tool.
        assert_eq!(
            Model::Default.model_name_for_tool(Tool::Copilot),
            Some("gpt-5-mini")
        );
        assert_eq!(
            Model::Default.model_name_for_tool(Tool::Claude),
            Some("claude-3-5-sonnet")
        );
    }

    #[test]
    fn model_parsing() {
        assert_eq!("gpt-5.2".parse::<Model>().unwrap(), Model::Gpt5_2);
        assert_eq!("GPT-5.2".parse::<Model>().unwrap(), Model::Gpt5_2);
        assert_eq!("gpt-5-2".parse::<Model>().unwrap(), Model::Gpt5_2);
        assert_eq!(
            "claude-sonnet-4.5".parse::<Model>().unwrap(),
            Model::ClaudeSonnet4_5
        );
        assert!("invalid-model".parse::<Model>().is_err());
    }
}
*/

// new tests for model-defaulting behaviour
#[cfg(test)]
mod comment_model_tests {
    use super::*;
    use crate::config::ZbobrDispatcherConfig;
    use crate::mcp::traits::CommonMcpImpl;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, atomic::AtomicU64};
    use tokio::sync::Mutex;

    /// Simple in-memory backend that records posted comments so tests can
    /// observe them.
    struct TrackingBackend {
        tasks: Mutex<HashMap<u64, Task>>,
        comments: Mutex<HashMap<u64, Vec<Comment>>>,
        next_id: AtomicU64,
    }

    #[async_trait]
    impl crate::backend::TaskBackend for TrackingBackend {
        async fn get_task(&self, id: u64) -> anyhow::Result<Task> {
            let tasks = self.tasks.lock().await;
            tasks
                .get(&id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("not found"))
        }

        async fn create_task(
            &self,
            title: &str,
            description: &str,
            stage: Stage,
            parameters: HashMap<Parameter, String>,
        ) -> anyhow::Result<u64> {
            let id = self
                .next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            let task = Task {
                id,
                title: title.to_string(),
                description: description.to_string(),
                stage,
                parameters,
                checklist: vec![],
                signal: None,
                conflict: false,
                pause: false,
                confirm: false,
                etag: None,
            };
            self.tasks.lock().await.insert(id, task);
            Ok(id)
        }

        async fn close_task(&self, _id: u64) -> anyhow::Result<()> {
            Ok(())
        }

        async fn is_task_closed(&self, _id: u64) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn modify_task(
            &self,
            id: u64,
            mutate: Box<dyn FnOnce(Task) -> Task + Send>,
        ) -> anyhow::Result<()> {
            let mut tasks = self.tasks.lock().await;
            if let Some(t) = tasks.remove(&id) {
                let t = mutate(t);
                tasks.insert(id, t);
                Ok(())
            } else {
                Err(anyhow::anyhow!("not found"))
            }
        }

        async fn list_tasks_by_stage(&self, _stage: Stage) -> anyhow::Result<Vec<Task>> {
            Ok(vec![])
        }

        async fn get_task_comments(&self, id: u64) -> anyhow::Result<Vec<Comment>> {
            let comments = self.comments.lock().await;
            Ok(comments.get(&id).cloned().unwrap_or_default())
        }

        async fn post_task_comment(
            &self,
            id: u64,
            comment_type: CommentType,
            role: Option<Role>,
            hostname: &str,
            tool: Option<Tool>,
            model: Option<Model>,
            body: &str,
        ) -> anyhow::Result<()> {
            let mut comments = self.comments.lock().await;
            comments.entry(id).or_default().push(Comment {
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

    struct DummyRepo;
    #[async_trait]
    impl crate::backend::WorktreeBackend for DummyRepo {
        async fn update_worktree(
            &self,
            _remote_repo: &str,
            _base_branch: &str,
            _work_branch: &str,
            _workspace_path: &std::path::Path,
        ) -> anyhow::Result<bool> {
            Ok(true)
        }

        async fn update_pr(&self, _work_branch: &str) -> anyhow::Result<String> {
            Ok("mock-pr-url".to_string())
        }

        async fn validate_connectivity(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn debug_state(&self) -> String {
            "dummy".to_string()
        }
    }

    fn make_dispatcher() -> crate::ZbobrDispatcherDyn {
        let backend: Arc<dyn crate::backend::TaskBackend> = Arc::new(TrackingBackend {
            tasks: Mutex::new(HashMap::new()),
            comments: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(0),
        });
        let repo: Arc<dyn crate::backend::WorktreeBackend> = Arc::new(DummyRepo);
        crate::ZbobrDispatcher::new_with_backends(ZbobrDispatcherConfig::default(), backend, repo)
    }

    #[tokio::test]
    async fn mcp_helper_includes_explicit_model() {
        let zbobr = make_dispatcher();
        let id = zbobr
            .create_task("t", "", Stage::Pending, None, None)
            .await
            .unwrap();

        // construct a planner MCP instance with a concrete model and use it to
        // report an error; the TrackingBackend should record the model field
        // from the MCP session rather than leaving it None.
        let planner =
            crate::mcp::planner::PlannerMcp::new(zbobr.clone(), id, Tool::Copilot, Model::Gpt5Mini);

        // call helper directly
        let _ = planner.report_error_impl("oops").await;

        let comments = zbobr.get_task_comments(id).await.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].model, Some(Model::Gpt5Mini));
    }

    #[tokio::test]
    async fn dispatcher_posts_have_no_model() {
        let zbobr = make_dispatcher();
        let id = zbobr
            .create_task("t", "", Stage::Pending, None, None)
            .await
            .unwrap();

        zbobr
            .task_session(id)
            .post_comment(
                CommentType::Error,
                "dispatcher error",
                None,
                "host",
                None,
                None,
            )
            .await
            .unwrap();

        let comments = zbobr.get_task_comments(id).await.unwrap();
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

        // verify default-model serialization
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

        // verify tool field serialization when both tool and model present
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
