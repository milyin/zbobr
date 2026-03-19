#![allow(dead_code)]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use zbobr_api::config::{PipelineConfig, StageDefinition};
use zbobr_dispatcher::{
    ChecklistItem, Comment, Task, TaskDir, ZbobrDispatcher,
    ZbobrDispatcherBuilder, ZbobrDispatcherConfig,
    backend::{TaskBackend, TaskBackendExt, WorktreeBackend},
    cli::process_task,
    prompts::ConfiguredPromptBuilder,
    task::Tool,
};
use zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterConfig;
use zbobr_repo_backend_fs::{ZbobrRepoBackendFs, ZbobrRepoBackendFsConfig};
use zbobr_repo_backend_github::{ZbobrRepoBackendGithub, ZbobrRepoBackendGithubConfig};
use zbobr_task_backend_fs::{ArcTaskBackendFs, ZbobrTaskBackendFs, ZbobrTaskBackendFsConfig};
use zbobr_task_backend_github::{TaskBackendGithub, ZbobrTaskBackendGithubConfig};

static SCENARIO_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Shared environment for integration tests.  Holds a live `ZbobrDispatcher`
/// and backends — no CLI binary involved.
pub struct IntegrationTestEnv {
    pub base_path: PathBuf,
    pub workspaces_dir: PathBuf,
    pub name: &'static str,
    pub zbobr: ZbobrDispatcher,
    pub task_backend: Arc<dyn TaskBackend>,
    pub repo_backend: Arc<dyn WorktreeBackend>,
    /// Optional remote repository slug (`owner/repo`) used by GitHub repo-backend tests.
    /// `None` for the filesystem repo backend.
    pub target_repo: Option<String>,
    /// Fork owner for the GitHub repo backend; `None` for the filesystem repo backend.
    fork_owner: Option<String>,
}

/// Construct an environment backed by two filesystem backends.
///
/// Returns `None` when `mcp-tester` is not installed (tests are skipped).
pub async fn init_fs_fs(name: &'static str) -> Option<Arc<IntegrationTestEnv>> {
    if !check_mcp_tester().await {
        return None;
    }

    let base_path = make_base_path(name).await;
    let workspaces_dir = base_path.join("workspaces");

    eprintln!(
        "[IntegrationTestEnv/{name}] base path: {}",
        base_path.display()
    );

    let dispatcher_config = ZbobrDispatcherConfig {
        workspaces: workspaces_dir.clone(),
        tool: Tool::McpTester,
        on_conflict: Some("merging".to_string()),
        ..ZbobrDispatcherConfig::default()
    };

    let task_backend_config = ZbobrTaskBackendFsConfig {
        tasks_dir: base_path.join("tasks"),
    };
    let repo_backend_config = ZbobrRepoBackendFsConfig {
        repos_dir: base_path.join("repos"),
    };

    let task_backend: Arc<dyn TaskBackend> = Arc::new(
        ArcTaskBackendFs::new(ZbobrTaskBackendFs::from_config(task_backend_config).ok()?),
    );
    let repo_backend: Arc<dyn WorktreeBackend> =
        Arc::new(ZbobrRepoBackendFs::from_config(repo_backend_config).ok()?);

    let zbobr = ZbobrDispatcherBuilder::new()
        .with_config(Arc::new(dispatcher_config))
        .with_task_backend(Arc::clone(&task_backend))
        .with_repo_backend(Arc::clone(&repo_backend))
        .with_prompt_builder(ConfiguredPromptBuilder::new(None, Arc::new(PipelineConfig { stages: vec![], roles: Default::default() })))
        .build();

    zbobr
        .setup_repository(&*task_backend, false)
        .await
        .ok()?;

    Some(Arc::new(IntegrationTestEnv {
        base_path,
        workspaces_dir,
        name,
        zbobr,
        task_backend,
        repo_backend,
        target_repo: None,
        fork_owner: None,
    }))
}

/// Construct an environment backed by a GitHub task backend and filesystem repo backend.
///
/// Returns `None` when `mcp-tester` is not installed (tests are skipped).
pub async fn init_github_fs(
    name: &'static str,
    task_repo: String,
    task_token: String,
) -> Option<Arc<IntegrationTestEnv>> {
    install_rustls_provider();
    if !check_mcp_tester().await {
        return None;
    }

    let base_path = make_base_path(name).await;
    let workspaces_dir = base_path.join("workspaces");

    eprintln!(
        "[IntegrationTestEnv/{name}] base path: {}",
        base_path.display()
    );

    let dispatcher_config = ZbobrDispatcherConfig {
        workspaces: workspaces_dir.clone(),
        tool: Tool::McpTester,
        ..ZbobrDispatcherConfig::default()
    };

    let task_backend_config = ZbobrTaskBackendGithubConfig {
        github_repo: task_repo.clone(),
        github_token: task_token,
    };
    let repo_backend_config = ZbobrRepoBackendFsConfig {
        repos_dir: base_path.join("repos"),
    };

    let task_backend: Arc<dyn TaskBackend> =
        Arc::new(TaskBackendGithub::from_config(task_backend_config).ok()?);
    let repo_backend: Arc<dyn WorktreeBackend> =
        Arc::new(ZbobrRepoBackendFs::from_config(repo_backend_config).ok()?);

    let zbobr = ZbobrDispatcherBuilder::new()
        .with_config(Arc::new(dispatcher_config))
        .with_task_backend(Arc::clone(&task_backend))
        .with_repo_backend(Arc::clone(&repo_backend))
        .with_prompt_builder(ConfiguredPromptBuilder::new(None, Arc::new(PipelineConfig { stages: vec![], roles: Default::default() })))
        .build();

    zbobr
        .setup_repository(&*task_backend, false)
        .await
        .ok()?;

    Some(Arc::new(IntegrationTestEnv {
        base_path,
        workspaces_dir,
        name,
        zbobr,
        task_backend,
        repo_backend,
        target_repo: Some(task_repo),
        fork_owner: None,
    }))
}

/// Construct an environment backed by a filesystem task backend and GitHub repo backend.
///
/// Returns `None` when `mcp-tester` is not installed (tests are skipped).
pub async fn init_fs_github(
    name: &'static str,
    fork_owner: String,
    repo_token: String,
    target_repo: Option<String>,
) -> Option<Arc<IntegrationTestEnv>> {
    install_rustls_provider();
    if !check_mcp_tester().await {
        return None;
    }

    let base_path = make_base_path(name).await;
    let workspaces_dir = base_path.join("workspaces");

    eprintln!(
        "[IntegrationTestEnv/{name}] base path: {}",
        base_path.display()
    );

    let dispatcher_config = ZbobrDispatcherConfig {
        workspaces: workspaces_dir.clone(),
        tool: Tool::McpTester,
        ..ZbobrDispatcherConfig::default()
    };

    let task_backend_config = ZbobrTaskBackendFsConfig {
        tasks_dir: base_path.join("tasks"),
    };
    let repo_backend_config = ZbobrRepoBackendGithubConfig {
        fork_owner: fork_owner.clone(),
        github_token: repo_token,
        repos_dir: base_path.join("repos"),
        git_user_name: "test-bot".to_string(),
        git_user_email: "test@example.com".to_string(),
        overwrite_author: false,
    };

    let task_backend: Arc<dyn TaskBackend> = Arc::new(
        ArcTaskBackendFs::new(ZbobrTaskBackendFs::from_config(task_backend_config).ok()?),
    );
    let repo_backend: Arc<dyn WorktreeBackend> =
        Arc::new(ZbobrRepoBackendGithub::from_config(repo_backend_config).ok()?);

    let zbobr = ZbobrDispatcherBuilder::new()
        .with_config(Arc::new(dispatcher_config))
        .with_task_backend(Arc::clone(&task_backend))
        .with_repo_backend(Arc::clone(&repo_backend))
        .with_prompt_builder(ConfiguredPromptBuilder::new(None, Arc::new(PipelineConfig { stages: vec![], roles: Default::default() })))
        .build();

    zbobr
        .setup_repository(&*task_backend, false)
        .await
        .ok()?;

    Some(Arc::new(IntegrationTestEnv {
        base_path,
        workspaces_dir,
        name,
        zbobr,
        task_backend,
        repo_backend,
        target_repo,
        fork_owner: Some(fork_owner),
    }))
}

/// Construct an environment backed by GitHub task and repo backends.
///
/// Returns `None` when `mcp-tester` is not installed (tests are skipped).
pub async fn init_github_github(
    name: &'static str,
    task_repo: String,
    task_token: String,
    fork_owner: String,
    repo_token: String,
) -> Option<Arc<IntegrationTestEnv>> {
    install_rustls_provider();
    if !check_mcp_tester().await {
        return None;
    }

    let base_path = make_base_path(name).await;
    let workspaces_dir = base_path.join("workspaces");

    eprintln!(
        "[IntegrationTestEnv/{name}] base path: {}",
        base_path.display()
    );

    let dispatcher_config = ZbobrDispatcherConfig {
        workspaces: workspaces_dir.clone(),
        tool: Tool::McpTester,
        ..ZbobrDispatcherConfig::default()
    };

    let task_backend_config = ZbobrTaskBackendGithubConfig {
        github_repo: task_repo.clone(),
        github_token: task_token,
    };
    let repo_backend_config = ZbobrRepoBackendGithubConfig {
        fork_owner: fork_owner.clone(),
        github_token: repo_token,
        repos_dir: base_path.join("repos"),
        git_user_name: "test-bot".to_string(),
        git_user_email: "test@example.com".to_string(),
        overwrite_author: false,
    };

    let task_backend: Arc<dyn TaskBackend> =
        Arc::new(TaskBackendGithub::from_config(task_backend_config).ok()?);
    let repo_backend: Arc<dyn WorktreeBackend> =
        Arc::new(ZbobrRepoBackendGithub::from_config(repo_backend_config).ok()?);

    let zbobr = ZbobrDispatcherBuilder::new()
        .with_config(Arc::new(dispatcher_config))
        .with_task_backend(Arc::clone(&task_backend))
        .with_repo_backend(Arc::clone(&repo_backend))
        .with_prompt_builder(ConfiguredPromptBuilder::new(None, Arc::new(PipelineConfig { stages: vec![], roles: Default::default() })))
        .build();

    zbobr
        .setup_repository(&*task_backend, false)
        .await
        .ok()?;

    Some(Arc::new(IntegrationTestEnv {
        base_path,
        workspaces_dir,
        name,
        zbobr,
        task_backend,
        repo_backend,
        target_repo: Some(task_repo),
        fork_owner: Some(fork_owner),
    }))
}

impl IntegrationTestEnv {
    // -----------------------------------------------------------------------
    // Identification
    // -----------------------------------------------------------------------

    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Return the fork owner for a GitHub repo backend, or `None` for FS.
    pub fn fork_owner(&self) -> Option<&str> {
        self.fork_owner.as_deref()
    }

    // -----------------------------------------------------------------------
    // Task utilities
    // -----------------------------------------------------------------------

    pub async fn create_task(&self, title: &str, description: &str, state: &str) -> u64 {
        self.zbobr
            .create_task(&*self.task_backend, title, description, state, None, None)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to create task: {e}", self.name))
    }

    pub async fn create_task_with_confirm(
        &self,
        title: &str,
        description: &str,
        state: &str,
        confirm: bool,
    ) -> u64 {
        self.zbobr
            .create_task_with_confirm(
                &*self.task_backend,
                title,
                description,
                state,
                None,
                None,
                confirm,
            )
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to create task: {e}", self.name))
    }

    pub async fn get_task(&self, task_id: u64) -> Task {
        self.task_backend
            .get_task(task_id)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to get task #{task_id}: {e}", self.name))
            .snapshot()
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to snapshot task #{task_id}: {e}", self.name))
    }

    pub async fn get_comments(&self, task_id: u64) -> Vec<Comment> {
        self.task_backend
            .get_task(task_id)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to get task #{task_id}: {e}", self.name))
            .get_comments()
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] failed to get comments for task #{task_id}: {e}",
                    self.name
                )
            })
    }

    pub async fn insert_checklist_item(&self, task_id: u64, id: &str, text: &str) {
        let id = id.to_string();
        let text = text.to_string();
        self.zbobr
            .task_session(
                Arc::clone(&self.task_backend),
                Arc::clone(&self.repo_backend),
                task_id,
            )
            .modify_task(move |mut task| {
                task.checklist.push(ChecklistItem {
                    id,
                    checked: false,
                    text,
                });
                task
            })
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] failed to insert checklist item for task #{task_id}: {e}",
                    self.name
                )
            });
    }

    pub async fn update_task_branches(
        &self,
        task_id: u64,
        dest_repo: &str,
        dest_branch: &str,
        work_branch: &str,
    ) {
        let dest_repo = dest_repo.to_string();
        let dest_branch = dest_branch.to_string();
        let work_branch = work_branch.to_string();
        let weak = self
            .task_backend
            .get_task(task_id)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to get task #{task_id}: {e}", self.name));
        let mutable = weak
            .upgrade()
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to upgrade task #{task_id}: {e}", self.name));
        mutable
            .modify_task(Box::new(move |mut task| {
                task.destination_repository = Some(dest_repo);
                task.destination_branch = Some(dest_branch);
                task.work_branch = Some(work_branch);
                task
            }))
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] failed to update task #{task_id} branches: {e}",
                    self.name
                )
            });
    }

    pub async fn update_task_signal(&self, task_id: u64, signal: &str) {
        self.task_backend
            .set_task_signal(task_id, Some(signal))
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] failed to set signal on task #{task_id}: {e}",
                    self.name
                )
            });
    }

    /// Update the task's state, simulating a manual state transition with
    /// respect to the `confirm` flag (sets `pause` when confirm is true).
    pub async fn update_task_state(&self, task_id: u64, new_state: &str) {
        let new_state = new_state.to_string();
        let weak = self
            .task_backend
            .get_task(task_id)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to get task #{task_id}: {e}", self.name));
        let mutable = weak
            .upgrade()
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to upgrade task #{task_id}: {e}", self.name));
        mutable
            .modify_task(Box::new(move |mut task| {
                if task.confirm && task.state != new_state {
                    task.pause = true;
                }
                task.state = new_state;
                task
            }))
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] failed to update task #{task_id} state: {e}",
                    self.name
                )
            });
    }

    // -----------------------------------------------------------------------
    // Git utilities
    // -----------------------------------------------------------------------

    pub async fn create_git_repo(&self, name: &str) -> PathBuf {
        let repo_dir = self.base_path.join(name);
        tokio::fs::create_dir_all(&repo_dir).await.unwrap();

        async fn git(dir: &PathBuf, args: &[&str]) {
            let status = tokio::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .status()
                .await
                .unwrap();
            assert!(status.success(), "git {:?} failed", args);
        }

        git(&repo_dir, &["init"]).await;
        git(&repo_dir, &["config", "user.name", "test-bot"]).await;
        git(&repo_dir, &["config", "user.email", "test@example.com"]).await;
        tokio::fs::write(repo_dir.join("README.md"), "test repo")
            .await
            .unwrap();
        git(&repo_dir, &["add", "README.md"]).await;
        git(&repo_dir, &["commit", "-m", "Initial commit"]).await;
        git(&repo_dir, &["branch", "-M", "main"]).await;

        repo_dir
    }

    // -----------------------------------------------------------------------
    // Workspace helpers
    // -----------------------------------------------------------------------

    /// Clone `repo_path` into a per-task workspace directory and check out
    /// `work_branch` (creating it if absent).  Returns the clone path.
    pub async fn prepare_workspace(
        &self,
        task_id: u64,
        repo_path: &Path,
        work_branch: &str,
    ) -> PathBuf {
        let task_dir = TaskDir::new(&self.workspaces_dir, task_id);
        tokio::fs::create_dir_all(task_dir.path())
            .await
            .expect("failed to create workspace dir");

        let repo_name = repo_path
            .file_name()
            .expect("repo_path must have a file name")
            .to_str()
            .unwrap();
        let work_dir = task_dir.path().join(repo_name);

        let clone_ok = tokio::process::Command::new("git")
            .args([
                "clone",
                repo_path.to_str().unwrap(),
                work_dir.to_str().unwrap(),
            ])
            .status()
            .await
            .expect("failed to run git clone")
            .success();
        assert!(clone_ok, "[{}] git clone failed", self.name);

        let checkout = tokio::process::Command::new("git")
            .args(["checkout", work_branch])
            .current_dir(&work_dir)
            .output()
            .await
            .expect("failed to run git checkout");

        if !checkout.status.success() {
            let ok = tokio::process::Command::new("git")
                .args(["checkout", "-b", work_branch])
                .current_dir(&work_dir)
                .status()
                .await
                .expect("failed to run git checkout -b")
                .success();
            assert!(
                ok,
                "[{}] failed to create branch '{work_branch}'",
                self.name
            );
        }

        work_dir
    }

    /// Clone `repo_slug` into a per-task workspace directory via the configured
    /// repo backend (`zbobr.clone_and_setup`), then check out `work_branch`
    /// (creating it if absent).  Returns the clone path.
    pub async fn prepare_workspace_via_repo_backend(
        &self,
        task_id: u64,
        _repo_slug: &str,
        _work_branch: &str,
    ) -> PathBuf {
        let task = self.get_task(task_id).await;
        let identity = task
            .identity()
            .unwrap_or_else(|| panic!("[{}] task #{task_id} missing routing params", self.name));

        self.zbobr
            .update_worktree(&*self.repo_backend, &identity)
            .await
            .unwrap_or_else(|e| panic!("[{}] update_worktree failed: {e}", self.name));

        let dest_repo = &identity.destination_repository;
        let repo_name = dest_repo.rsplit('/').next().unwrap_or(dest_repo);
        let task_dir = TaskDir::new(self.zbobr.config().workspaces.as_path(), task_id);
        let work_dir = task_dir.path().join(repo_name);

        assert!(
            work_dir.exists(),
            "[{}] Workspace directory missing after update: {}",
            self.name,
            work_dir.display()
        );

        work_dir
    }

    // -----------------------------------------------------------------------
    // Stage runner
    // -----------------------------------------------------------------------

    /// Run the dispatcher with a full pipeline config and per-role scenario map.
    ///
    /// Unlike `run_stage`, this accepts an arbitrary `PipelineConfig` with any
    /// stage/role/mode names. Scenarios are mapped via the generic `scenarios`
    /// HashMap in the mcp-tester config.
    pub async fn run_pipeline(
        &self,
        task_id: u64,
        pipeline: &PipelineConfig,
        role_scenarios: &HashMap<String, String>,
    ) {
        self.task_backend
            .set_task_state(task_id, "READY")
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to set task state: {e}", self.name));
        self.task_backend
            .set_task_stack(task_id, vec![])
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to clear task stack: {e}", self.name));

        let idx = SCENARIO_COUNTER.fetch_add(1, Ordering::Relaxed);
        let scenarios_dir = self.base_path.join("scenarios").join(format!("{idx}"));
        tokio::fs::create_dir_all(&scenarios_dir)
            .await
            .expect("failed to create scenarios directory");

        let mut scenario_paths: HashMap<String, std::path::PathBuf> = HashMap::new();
        for (role, yaml) in role_scenarios {
            let path = scenarios_dir.join(format!("{role}.yml"));
            tokio::fs::write(&path, yaml)
                .await
                .expect("failed to write scenario file");
            scenario_paths.insert(role.clone(), path);
        }

        let mcp_tester_config = ZbobrExecutorMcpTesterConfig {
            scenarios: scenario_paths,
            ..Default::default()
        };

        let stage_dispatcher = self.zbobr.with_mcp_tester_config(mcp_tester_config);

        let task = self.get_task(task_id).await;

        process_task(&stage_dispatcher, &task, pipeline)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] process_task failed for task #{task_id}: {e}",
                    self.name
                )
            });
    }

    /// Keep calling `process_task` until the task is no longer actionable
    /// (DONE, PAUSE, or no progress).  Returns the number of iterations.
    pub async fn run_to_completion(
        &self,
        task_id: u64,
        pipeline: &PipelineConfig,
        role_scenarios: &HashMap<String, String>,
        max_iterations: usize,
    ) -> usize {
        for i in 0..max_iterations {
            let task = self.get_task(task_id).await;
            if task.state == "DONE" || task.state == "PAUSE" {
                return i;
            }
            if task.pause {
                return i;
            }
            // If state is PENDING but no signal, nothing to do
            if task.state.ends_with("_PENDING") && task.signal.is_none() {
                return i;
            }

            let idx = SCENARIO_COUNTER.fetch_add(1, Ordering::Relaxed);
            let scenarios_dir = self.base_path.join("scenarios").join(format!("{idx}"));
            tokio::fs::create_dir_all(&scenarios_dir)
                .await
                .expect("failed to create scenarios directory");

            let mut scenario_paths: HashMap<String, std::path::PathBuf> = HashMap::new();
            for (role, yaml) in role_scenarios {
                let path = scenarios_dir.join(format!("{role}.yml"));
                tokio::fs::write(&path, yaml)
                    .await
                    .expect("failed to write scenario file");
                scenario_paths.insert(role.clone(), path);
            }

            let mcp_tester_config = ZbobrExecutorMcpTesterConfig {
                scenarios: scenario_paths,
                ..Default::default()
            };

            let stage_dispatcher = self.zbobr.with_mcp_tester_config(mcp_tester_config);
            let task = self.get_task(task_id).await;

            process_task(&stage_dispatcher, &task, pipeline)
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "[{}] process_task iteration {i} failed for task #{task_id}: {e}",
                        self.name
                    )
                });
        }
        max_iterations
    }

    /// Continue processing a task with the same pipeline and scenarios.
    ///
    /// Unlike `run_pipeline`, this does NOT reset state/stack — it resumes
    /// from the current task state (e.g. after a signal transition set state
    /// to `{mode}_PENDING`).
    pub async fn continue_pipeline(
        &self,
        task_id: u64,
        pipeline: &PipelineConfig,
        role_scenarios: &HashMap<String, String>,
    ) {
        let idx = SCENARIO_COUNTER.fetch_add(1, Ordering::Relaxed);
        let scenarios_dir = self.base_path.join("scenarios").join(format!("{idx}"));
        tokio::fs::create_dir_all(&scenarios_dir)
            .await
            .expect("failed to create scenarios directory");

        let mut scenario_paths: HashMap<String, std::path::PathBuf> = HashMap::new();
        for (role, yaml) in role_scenarios {
            let path = scenarios_dir.join(format!("{role}.yml"));
            tokio::fs::write(&path, yaml)
                .await
                .expect("failed to write scenario file");
            scenario_paths.insert(role.clone(), path);
        }

        let mcp_tester_config = ZbobrExecutorMcpTesterConfig {
            scenarios: scenario_paths,
            ..Default::default()
        };

        let stage_dispatcher = self.zbobr.with_mcp_tester_config(mcp_tester_config);

        let task = self.get_task(task_id).await;

        process_task(&stage_dispatcher, &task, pipeline)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] process_task (continue) failed for task #{task_id}: {e}",
                    self.name
                )
            });
    }

    /// Run the dispatcher for the given role against `task_id`, using the
    /// provided scenario YAML string as the mcp-tester script.
    ///
    /// Internally:
    ///  1. Sets the task's state to `"READY"` so the pipeline dispatches this role.
    ///  2. Writes the scenario to a temporary file.
    ///  3. Builds a single-stage pipeline for the given role.
    ///  4. Overrides dispatcher MCP tester config with the scenario file for the role.
    ///  5. Calls `process_task` directly (no subprocess).
    pub async fn run_stage(&self, task_id: u64, role: &str, scenario: String) {
        self.task_backend
            .set_task_state(task_id, "READY")
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to set task state: {e}", self.name));
        // Clear leftover stack from previous stages so the state machine
        // dispatches cleanly from the start stage.  Signal is preserved so
        // that tests which set a signal before calling run_stage can verify
        // the entry/exit behavior.
        self.task_backend
            .set_task_stack(task_id, vec![])
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to clear task stack: {e}", self.name));

        let idx = SCENARIO_COUNTER.fetch_add(1, Ordering::Relaxed);
        let scenarios_dir = self.base_path.join("scenarios").join(format!("{idx}"));
        tokio::fs::create_dir_all(&scenarios_dir)
            .await
            .expect("failed to create scenarios directory");
        let scenario_path = scenarios_dir.join("scenario.yml");
        tokio::fs::write(&scenario_path, &scenario)
            .await
            .expect("failed to write scenario file");

        let mcp_tester_config = match role {
            "preparator" => ZbobrExecutorMcpTesterConfig {
                preparation: Some(scenario_path),
                ..Default::default()
            },
            "planner" => ZbobrExecutorMcpTesterConfig {
                planning: Some(scenario_path),
                ..Default::default()
            },
            "worker" => ZbobrExecutorMcpTesterConfig {
                working: Some(scenario_path),
                ..Default::default()
            },
            "reviewer" => ZbobrExecutorMcpTesterConfig {
                reviewing: Some(scenario_path),
                ..Default::default()
            },
            "tester" => ZbobrExecutorMcpTesterConfig {
                testing: Some(scenario_path),
                ..Default::default()
            },
            "merger" => ZbobrExecutorMcpTesterConfig {
                merging: Some(scenario_path),
                ..Default::default()
            },
            other => panic!("[{}] unknown role: {other}", self.name),
        };

        // Build a minimal single-stage pipeline for the given role so that
        // process_task dispatches exactly this role when state is "READY".
        // Transitions mirror the old hardcoded post-stage signal behavior.
        let stage_name = match role {
            "preparator" => "preparing",
            "planner" => "planning",
            "worker" => "working",
            "reviewer" => "reviewing",
            "tester" => "testing",
            "merger" => "merging",
            other => panic!("[{}] unknown role for stage name: {other}", self.name),
        }
        .to_string();
        let transitions: HashMap<String, String> = match role {
            "preparator" => [("default".into(), "go_planning".into())].into(),
            "planner" => [("default".into(), "go_working".into())].into(),
            "worker" => [("default".into(), "go_reviewing".into())].into(),
            "reviewer" => [
                ("review_accept".into(), "go_testing".into()),
                ("review_reject".into(), "go_planning".into()),
                ("default".into(), "go_testing".into()),
            ]
            .into(),
            "tester" => [
                ("test_accept".into(), "return".into()),
                ("test_reject".into(), "go_planning".into()),
                ("default".into(), "return".into()),
            ]
            .into(),
            "merger" => [("default".into(), "return".into())].into(),
            other => panic!("[{}] unknown role for transitions: {other}", self.name),
        };
        let pipeline = PipelineConfig {
            stages: vec![StageDefinition {
                name: stage_name.clone(),
                role: role.to_string(),
                model: None,
                tool: Some(Tool::McpTester),
                main_prompt: None,
                additional_prompts: vec![],
                transitions,
                is_start: true,
                mode: "main".to_string(),
            }],
            roles: Default::default(),
        };

        let stage_dispatcher = self.zbobr.with_mcp_tester_config(mcp_tester_config);

        let task = self.get_task(task_id).await;

        process_task(&stage_dispatcher, &task, &pipeline)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] process_task failed for task #{task_id}: {e}",
                    self.name
                )
            });
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn install_rustls_provider() {
    // Octocrab (via reqwest/rustls) requires a global CryptoProvider to be
    // installed before any HTTPS connection is made.  Installing the ring
    // provider is idempotent; the error is silently ignored if another caller
    // already installed one.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

async fn check_mcp_tester() -> bool {
    let result = tokio::process::Command::new("mcp-tester")
        .arg("--version")
        .output()
        .await;
    match result {
        Ok(out) if out.status.success() => true,
        _ => {
            eprintln!(
                "Skipping integration tests: mcp-tester not installed \
                 (cargo install mcp-tester)"
            );
            false
        }
    }
}

async fn make_base_path(name: &str) -> PathBuf {
    let base = match std::env::var("CARGO_TARGET_TMPDIR") {
        Ok(p) => PathBuf::from(p).join(format!("integration_env_{name}")),
        Err(_) => std::env::temp_dir().join(format!("zbobr_integration_env_{name}")),
    };
    if base.exists() {
        tokio::fs::remove_dir_all(&base)
            .await
            .expect("failed to clean previous integration env");
    }
    tokio::fs::create_dir_all(&base)
        .await
        .expect("failed to create integration env base dir");
    base
}
