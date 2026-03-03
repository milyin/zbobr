pub use zbobr_api::task::*;

use crate::ZbobrDispatcherDyn;

// ---------------------------------------------------------------------------
// RoleSession — restricted access for MCP tools during agent sessions.
//
// Cannot modify: stage, conflict, confirm (those are dispatcher-only transitions).
// Can modify: description, plan, checklist, parameters, signal, pause.
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

    /// Get the current task plan.
    pub async fn get_plan(&self) -> anyhow::Result<String> {
        Ok(self.get_task().await?.plan)
    }

    /// Get the current task checklist.
    pub async fn get_checklist(&self) -> anyhow::Result<Vec<ChecklistItem>> {
        Ok(self.get_task().await?.checklist)
    }

    /// Atomically read-modify-write the task body.
    ///
    /// The closure receives a mutable `Task` reference and may modify `description`,
    /// `parameters`, `plan`, `checklist`, `signal`, and `pause`.
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

    /// Get all discussion messages on the task (structured format, filtered to Request type only).
    pub async fn get_discussion(&self) -> anyhow::Result<Vec<Comment>> {
        let all_comments = self.zbobr.get_task_comments_structured(self.task_id).await?;
        Ok(all_comments
            .into_iter()
            .filter(|c| c.comment_type == CommentType::Request)
            .collect())
    }

    /// Get all comments as structured Comment objects (includes all types: error, report, reply).
    pub async fn get_history(&self) -> anyhow::Result<Vec<Comment>> {
        self.zbobr.get_task_comments_structured(self.task_id).await
    }

    pub async fn post_message_structured(
        &self,
        comment_type: CommentType,
        body: &str,
        role: Option<Role>,
        hostname: &str,
        model: Option<Model>,
    ) -> anyhow::Result<()> {
        self.zbobr
            .post_task_comment_structured(self.task_id, comment_type, role, hostname, model, body)
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

    /// Clone target repo and checkout specific branch (read-only, for planner).
    pub async fn request_branch_readonly(
        &self,
        repo: &str,
        branch: &str,
    ) -> anyhow::Result<String> {
        let path = self
            .zbobr
            .clone_readonly(repo, branch, self.task_id)
            .await?;
        let path_str = path.to_string_lossy().to_string();
        Ok(path_str)
    }

    /// Fork target repo, clone locally, checkout specific branch (for worker).
    pub async fn request_branch(&self, repo: &str, branch: &str) -> anyhow::Result<String> {
        let task = self.zbobr.get_task(self.task_id).await?;
        let destination_branch = task
            .parameters
            .get(&crate::Parameter::DestinationBranch)
            .cloned()
            .unwrap_or_else(|| "main".to_string());
        let path = self
            .zbobr
            .clone_and_setup(repo, branch, &destination_branch, self.task_id)
            .await?;
        let path_str = path.to_string_lossy().to_string();
        Ok(path_str)
    }

    /// Helper: Clone repo and checkout branch from PR.
    /// PR format: "https://github.com/owner/repo/pull/123" or "owner/repo#123"
    pub async fn request_branch_by_pr(&self, pr: &str, readonly: bool) -> anyhow::Result<String> {
        let (repo, branch) = self.zbobr.parse_pr_to_repo_branch(pr).await?;
        if readonly {
            self.request_branch_readonly(&repo, &branch).await
        } else {
            self.request_branch(&repo, &branch).await
        }
    }

    /// Push the current branch to the fork remote.
    /// Validates that the current branch has the correct task prefix.
    pub async fn push_branch(&self, path: &str) -> anyhow::Result<()> {
        let work_dir = std::path::PathBuf::from(path);

        if !work_dir.exists() {
            anyhow::bail!("Work directory does not exist: {}", work_dir.display());
        }

        // Get current branch name
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&work_dir)
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Failed to get current branch");
        }

        let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if !self.validate_branch_prefix(&current_branch) {
            anyhow::bail!(
                "Branch '{}' does not match expected prefix '{}/{}/'. Use create_branch_name to generate a valid branch name.",
                current_branch,
                self.zbobr.config().work_branch_prefix,
                self.task_id
            );
        }

        // Push to fork
        tracing::info!("Pushing branch '{}' to fork", current_branch);
        let status = tokio::process::Command::new("git")
            .args(["push", "-u", "fork", "HEAD", "--force"])
            .current_dir(&work_dir)
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("Failed to push to fork");
        }

        Ok(())
    }

    /// Push the branch and create PR within the fork.
    /// The PR is created in the fork repo with `destination_branch` as base.
    pub async fn push_branch_and_create_pr(
        &self,
        path: &str,
        destination_branch: &str,
    ) -> anyhow::Result<String> {
        // First push the branch
        self.push_branch(path).await?;

        let work_dir = std::path::PathBuf::from(path);

        // Get current branch name (already validated by push_branch)
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&work_dir)
            .output()
            .await?;
        let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Derive repository name from work directory name (workspace/task#/repo)
        let repo_name = work_dir
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine repo name from path: {}", path))?
            .to_string();

        // Build PR metadata from task (decoupled from repo backend)
        let task = self.get_task().await?;
        let pr_title = format!("Fix #{}: {}", self.task_id, task.title);
        let pr_body = format!(
            "Resolves #{}\n\nImplementation for: {}",
            self.task_id, task.title
        );

        // Create PR using the backend (which knows the fork owner)
        let pr_url = self
            .zbobr
            .create_pr_in_fork(
                &repo_name,
                &current_branch,
                destination_branch,
                &pr_title,
                &pr_body,
            )
            .await?;
        Ok(pr_url)
    }

    /// Ensure `pr_url` is stored in task parameters.
    ///
    /// If already set, returns the existing value immediately.
    /// If not set: calls `ensure_branch_and_pr` on the repo backend, stores the
    /// resulting URL in `Parameter::PrUrl`, and returns it.
    pub async fn ensure_pr_url(&self) -> anyhow::Result<String> {
        let task = self.get_task().await?;
        if let Some(url) = task.parameters.get(&Parameter::PrUrl).cloned() {
            return Ok(url);
        }
        let dest_repo = task
            .parameters
            .get(&Parameter::DestinationRepository)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("destination_repository parameter is not set"))?;
        let dest_branch = task
            .parameters
            .get(&Parameter::DestinationBranch)
            .cloned()
            .unwrap_or_else(|| "main".to_string());
        let work_branch = task
            .parameters
            .get(&Parameter::WorkBranch)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("work_branch parameter is not set"))?;
        let pr_title = format!("Fix #{}: {}", self.task_id, task.title);

        let pr_url = self
            .zbobr
            .ensure_branch_and_pr(
                &dest_repo,
                self.task_id,
                &work_branch,
                &dest_branch,
                &pr_title,
            )
            .await?;

        self.set_parameter(Parameter::PrUrl, Some(pr_url.clone()))
            .await?;
        Ok(pr_url)
    }

    /// Push current work branch commits to the remote.
    ///
    /// Reads `DestinationRepository` and `WorkBranch` from task parameters and delegates
    /// to the repo backend. FS backend is a no-op; GitHub backend performs a git push.
    pub async fn push_branch_commits(&self) -> anyhow::Result<()> {
        let task = self.get_task().await?;
        let dest_repo = task
            .parameters
            .get(&Parameter::DestinationRepository)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("destination_repository parameter is not set"))?;
        let work_branch = task
            .parameters
            .get(&Parameter::WorkBranch)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("work_branch parameter is not set"))?;
        self.zbobr
            .push_branch(&dest_repo, self.task_id, &work_branch)
            .await
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

    /// Mark task as done: set stage to Done and clear signal.
    pub async fn mark_done(&self) -> anyhow::Result<()> {
        self.modify_task(move |task| {
            task.stage = Stage::Done;
            task.signal = None;
        })
        .await
    }

    /// Post a structured comment with type, body, and optional role/model metadata.
    pub async fn post_message_structured(
        &self,
        comment_type: CommentType,
        body: &str,
        role: Option<Role>,
        hostname: &str,
        model: Option<Model>,
    ) -> anyhow::Result<()> {
        self.zbobr
            .post_task_comment_structured(self.task_id, comment_type, role, hostname, model, body)
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
    fn task_serde() {
        let task = Task {
            id: 42,
            title: "Test task".to_string(),
            description: "Do something".to_string(),
            plan: String::new(),
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
            plan: String::new(),
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
            plan: String::new(),
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
                plan: String::new(),
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
            _role: &str,
            _hostname: &str,
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
    impl crate::backend::RepoBackend for DummyRepo {
        async fn clone_and_setup(
            &self,
            _target_repo: &str,
            _work_branch: &str,
            _destination_branch: &str,
            _workspace_path: &std::path::Path,
        ) -> anyhow::Result<std::path::PathBuf> {
            unreachable!()
        }
        async fn clone_readonly(
            &self,
            _target_repo: &str,
            _branch: &str,
            _workspace_path: &std::path::Path,
        ) -> anyhow::Result<std::path::PathBuf> {
            unreachable!()
        }
        async fn setup_fork_remote_and_push(
            &self,
            _work_dir: &std::path::Path,
            _target_repo: &str,
            _work_branch: &str,
        ) -> anyhow::Result<()> {
            unreachable!()
        }
        async fn ensure_branch_and_pr(
            &self,
            _target_repo: &str,
            _workspace_path: &std::path::Path,
            _work_branch: &str,
            _destination_branch: &str,
            _pr_title: &str,
        ) -> anyhow::Result<String> {
            unreachable!()
        }
        async fn push_branch(
            &self,
            _target_repo: &str,
            _workspace_path: &std::path::Path,
            _work_branch: &str,
        ) -> anyhow::Result<()> {
            unreachable!()
        }
        async fn create_pr_in_fork(
            &self,
            _repo_name: &str,
            _work_branch: &str,
            _destination_branch: &str,
            _pr_title: &str,
            _pr_body: &str,
        ) -> anyhow::Result<String> {
            unreachable!()
        }
        async fn parse_pr_to_repo_branch(&self, _pr_ref: &str) -> anyhow::Result<(String, String)> {
            unreachable!()
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
        let repo: Arc<dyn crate::backend::RepoBackend> = Arc::new(DummyRepo);
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
            Some("sonnet")
        );
        assert_eq!(Model::Gpt5_2.model_name_for_tool(Tool::Claude), None);
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
