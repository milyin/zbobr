#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use zbobr_dispatcher::{
    ZbobrDispatcher, ZbobrDispatcherConfig, ZbobrDispatcherDyn, ZbobrExecutorConfig,
    Comment, Signal, Stage, Task, process_task_by_stage,
    prompts::Prompts,
    task::{Parameter, Tool},
};
use zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterConfig;
use zbobr_repo_backend_fs::{ZbobrRepoBackendFs, ZbobrRepoBackendFsConfig};
use zbobr_repo_backend_github::{ZbobrRepoBackendGithub, ZbobrRepoBackendGithubConfig};
use zbobr_task_backend_fs::{ZbobrTaskBackendFs, ZbobrTaskBackendFsConfig};
use zbobr_task_backend_github::{ZbobrTaskBackendGithub, ZbobrTaskBackendGithubConfig};

static SCENARIO_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Shared environment for integration tests.  Holds a live `ZbobrDispatcher`
/// backed by in-process backends — no CLI binary involved.
pub struct IntegrationTestEnv {
    pub base_path: PathBuf,
    pub workspaces_dir: PathBuf,
    pub name: &'static str,
    pub zbobr: ZbobrDispatcherDyn,
    /// Optional remote repository slug (`owner/repo`) used by GitHub repo-backend tests.
    /// `None` for the filesystem repo backend.
    pub target_repo: Option<String>,
    /// Fork owner for the GitHub repo backend; `None` for the filesystem repo backend.
    fork_owner: Option<String>,
}

impl IntegrationTestEnv {
    /// Construct an environment backed by two filesystem backends.
    ///
    /// Returns `None` when `mcp-tester` is not installed (tests are skipped).
    pub async fn init_fs_fs(name: &'static str) -> Option<Arc<Self>> {
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
            cli_tool: Tool::McpTester,
            git_user_name: "test-bot".to_string(),
            git_user_email: "test@example.com".to_string(),
            preparator_prompts: vec![],
            planner_prompts: vec![],
            worker_prompts: vec![],
            reviewer_prompts: vec![],
            merger_prompts: vec![],
            ..ZbobrDispatcherConfig::default()
        };

        let task_backend_config = ZbobrTaskBackendFsConfig {
            tasks_dir: base_path.join("tasks"),
        };
        let repo_backend_config = ZbobrRepoBackendFsConfig {
            repos_dir: base_path.join("repos"),
        };

        let task_backend: Arc<dyn zbobr_dispatcher::backend::TaskBackend> =
            Arc::new(ZbobrTaskBackendFs::from_config(task_backend_config).ok()?);
        let repo_backend: Arc<dyn zbobr_dispatcher::backend::RepoBackend> =
            Arc::new(ZbobrRepoBackendFs::from_config(repo_backend_config).ok()?);

        let zbobr =
            ZbobrDispatcher::new_with_backends(dispatcher_config, task_backend, repo_backend);

        zbobr.setup_repository(false).await.ok()?;

        Some(Arc::new(IntegrationTestEnv {
            base_path,
            workspaces_dir,
            name,
            zbobr,
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
    ) -> Option<Arc<Self>> {
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
            cli_tool: Tool::McpTester,
            git_user_name: "test-bot".to_string(),
            git_user_email: "test@example.com".to_string(),
            preparator_prompts: vec![],
            planner_prompts: vec![],
            worker_prompts: vec![],
            reviewer_prompts: vec![],
            merger_prompts: vec![],
            ..ZbobrDispatcherConfig::default()
        };

        let task_backend_config = ZbobrTaskBackendGithubConfig {
            github_repo: task_repo.clone(),
            github_token: task_token,
        };
        let repo_backend_config = ZbobrRepoBackendFsConfig {
            repos_dir: base_path.join("repos"),
        };

        let task_backend: Arc<dyn zbobr_dispatcher::backend::TaskBackend> =
            Arc::new(ZbobrTaskBackendGithub::from_config(task_backend_config).ok()?);
        let repo_backend: Arc<dyn zbobr_dispatcher::backend::RepoBackend> =
            Arc::new(ZbobrRepoBackendFs::from_config(repo_backend_config).ok()?);

        let zbobr =
            ZbobrDispatcher::new_with_backends(dispatcher_config, task_backend, repo_backend);

        zbobr.setup_repository(false).await.ok()?;

        Some(Arc::new(IntegrationTestEnv {
            base_path,
            workspaces_dir,
            name,
            zbobr,
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
    ) -> Option<Arc<Self>> {
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
            cli_tool: Tool::McpTester,
            git_user_name: "test-bot".to_string(),
            git_user_email: "test@example.com".to_string(),
            preparator_prompts: vec![],
            planner_prompts: vec![],
            worker_prompts: vec![],
            reviewer_prompts: vec![],
            merger_prompts: vec![],
            ..ZbobrDispatcherConfig::default()
        };

        let task_backend_config = ZbobrTaskBackendFsConfig {
            tasks_dir: base_path.join("tasks"),
        };
        let repo_backend_config = ZbobrRepoBackendGithubConfig {
            fork_owner: fork_owner.clone(),
            github_token: repo_token,
        };

        let task_backend: Arc<dyn zbobr_dispatcher::backend::TaskBackend> =
            Arc::new(ZbobrTaskBackendFs::from_config(task_backend_config).ok()?);
        let repo_backend: Arc<dyn zbobr_dispatcher::backend::RepoBackend> = Arc::new(
            ZbobrRepoBackendGithub::from_config(
                repo_backend_config,
                "test-bot".to_string(),
                "test@example.com".to_string(),
            )
            .ok()?,
        );

        let zbobr =
            ZbobrDispatcher::new_with_backends(dispatcher_config, task_backend, repo_backend);

        zbobr.setup_repository(false).await.ok()?;

        Some(Arc::new(IntegrationTestEnv {
            base_path,
            workspaces_dir,
            name,
            zbobr,
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
    ) -> Option<Arc<Self>> {
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
            cli_tool: Tool::McpTester,
            git_user_name: "test-bot".to_string(),
            git_user_email: "test@example.com".to_string(),
            preparator_prompts: vec![],
            planner_prompts: vec![],
            worker_prompts: vec![],
            reviewer_prompts: vec![],
            merger_prompts: vec![],
            ..ZbobrDispatcherConfig::default()
        };

        let task_backend_config = ZbobrTaskBackendGithubConfig {
            github_repo: task_repo.clone(),
            github_token: task_token,
        };
        let repo_backend_config = ZbobrRepoBackendGithubConfig {
            fork_owner: fork_owner.clone(),
            github_token: repo_token,
        };

        let task_backend: Arc<dyn zbobr_dispatcher::backend::TaskBackend> =
            Arc::new(ZbobrTaskBackendGithub::from_config(task_backend_config).ok()?);
        let repo_backend: Arc<dyn zbobr_dispatcher::backend::RepoBackend> = Arc::new(
            ZbobrRepoBackendGithub::from_config(
                repo_backend_config,
                "test-bot".to_string(),
                "test@example.com".to_string(),
            )
            .ok()?,
        );

        let zbobr =
            ZbobrDispatcher::new_with_backends(dispatcher_config, task_backend, repo_backend);

        zbobr.setup_repository(false).await.ok()?;

        Some(Arc::new(IntegrationTestEnv {
            base_path,
            workspaces_dir,
            name,
            zbobr,
            target_repo: Some(task_repo),
            fork_owner: Some(fork_owner),
        }))
    }

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

    pub async fn create_task(&self, title: &str, description: &str, stage: Stage) -> u64 {
        self.zbobr
            .create_task(title, description, stage, None, None, None, None)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to create task: {e}", self.name))
    }

    pub async fn create_task_with_confirm(
        &self,
        title: &str,
        description: &str,
        stage: Stage,
        confirm: bool,
    ) -> u64 {
        self.zbobr
            .create_task_with_confirm(
                title,
                description,
                stage,
                None,
                None,
                None,
                None,
                confirm,
            )
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to create task: {e}", self.name))
    }

    pub async fn get_task(&self, task_id: u64) -> Task {
        self.zbobr
            .get_task(task_id)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to get task #{task_id}: {e}", self.name))
    }

    pub async fn get_comments(&self, task_id: u64) -> Vec<Comment> {
        self.zbobr
            .get_task_comments_structured(task_id)
            .await
            .unwrap_or_else(|e| {
                panic!("[{}] failed to get comments for task #{task_id}: {e}", self.name)
            })
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
        self.zbobr
            .modify_task(
                task_id,
                Box::new(move |mut task| {
                    task.parameters
                        .insert(Parameter::DestinationRepository, dest_repo);
                    task.parameters
                        .insert(Parameter::DestinationBranch, dest_branch);
                    task.parameters.insert(Parameter::WorkBranch, work_branch);
                    task
                }),
            )
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] failed to update task #{task_id} branches: {e}",
                    self.name
                )
            });
    }

    pub async fn set_task_conflict(&self, task_id: u64, conflict: bool) {
        self.zbobr
            .modify_task(
                task_id,
                Box::new(move |mut task| {
                    task.conflict = conflict;
                    task
                }),
            )
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] failed to set conflict on task #{task_id}: {e}",
                    self.name
                )
            });
    }

    pub async fn update_task_signal(&self, task_id: u64, signal: &str) {
        let signal: Signal = signal
            .parse()
            .unwrap_or_else(|_| panic!("[{}] invalid signal '{signal}'", self.name));
        self.zbobr
            .set_task_signal(task_id, Some(signal))
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] failed to set signal on task #{task_id}: {e}",
                    self.name
                )
            });
    }

    /// Update the task's stage, simulating a manual stage transition with
    /// respect to the `confirm` flag (sets `pause` when confirm is true).
    pub async fn update_task_stage(&self, task_id: u64, new_stage: Stage) {
        self.zbobr
            .modify_task(
                task_id,
                Box::new(move |mut task| {
                    if task.confirm && task.stage != new_stage {
                        task.pause = true;
                    }
                    task.stage = new_stage;
                    task
                }),
            )
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] failed to update task #{task_id} stage: {e}",
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
        let workspace_dir = self.workspaces_dir.join(format!("task#{task_id}"));
        tokio::fs::create_dir_all(&workspace_dir)
            .await
            .expect("failed to create workspace dir");

        let repo_name = repo_path
            .file_name()
            .expect("repo_path must have a file name")
            .to_str()
            .unwrap();
        let work_dir = workspace_dir.join(repo_name);

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
        repo_slug: &str,
        work_branch: &str,
    ) -> PathBuf {
        let task = self.get_task(task_id).await;
        let dest_repo = task
            .parameters
            .get(&Parameter::DestinationRepository)
            .cloned()
            .unwrap_or_else(|| repo_slug.to_string());
        let dest_branch = task
            .parameters
            .get(&Parameter::DestinationBranch)
            .cloned()
            .unwrap_or_else(|| "main".to_string());

        let work_dir = self
            .zbobr
            .clone_and_setup(&dest_repo, work_branch, &dest_branch, task_id)
            .await
            .unwrap_or_else(|e| panic!("[{}] clone_and_setup failed: {e}", self.name));

        assert!(
            work_dir.exists(),
            "[{}] Workspace directory missing after clone: {}",
            self.name,
            work_dir.display()
        );

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
                "[{}] failed to create branch '{}' in workspace",
                self.name, work_branch
            );
        }

        work_dir
    }

    // -----------------------------------------------------------------------
    // Stage runner
    // -----------------------------------------------------------------------

    /// Run the dispatcher for the given stage against `task_id`, using the
    /// provided scenario YAML string as the mcp-tester script.
    ///
    /// Internally:
    ///  1. Sets the task's stage to `stage`.
    ///  2. Writes the scenario to a temporary file.
    ///  3. Builds a `ZbobrExecutorConfig` with the scenario file for `stage`.
    ///  4. Calls `process_task_by_stage` directly (no subprocess).
    pub async fn run_stage(&self, task_id: u64, stage: Stage, scenario: String) {
        self.zbobr
            .set_task_stage(task_id, stage)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to set stage: {e}", self.name));

        let idx = SCENARIO_COUNTER.fetch_add(1, Ordering::Relaxed);
        let scenarios_dir = self
            .base_path
            .join("scenarios")
            .join(format!("{idx}"));
        tokio::fs::create_dir_all(&scenarios_dir)
            .await
            .expect("failed to create scenarios directory");
        let scenario_path = scenarios_dir.join("scenario.yml");
        tokio::fs::write(&scenario_path, &scenario)
            .await
            .expect("failed to write scenario file");

        let mcp_tester_config = match stage {
            Stage::Preparing => ZbobrExecutorMcpTesterConfig {
                preparation: Some(scenario_path),
                ..Default::default()
            },
            Stage::Planning => ZbobrExecutorMcpTesterConfig {
                planning: Some(scenario_path),
                ..Default::default()
            },
            Stage::Working => ZbobrExecutorMcpTesterConfig {
                working: Some(scenario_path),
                ..Default::default()
            },
            Stage::Reviewing => ZbobrExecutorMcpTesterConfig {
                reviewing: Some(scenario_path),
                ..Default::default()
            },
            Stage::Merging => ZbobrExecutorMcpTesterConfig {
                merging: Some(scenario_path),
                ..Default::default()
            },
            other => panic!("[{}] run_stage: unsupported stage {:?}", self.name, other),
        };

        let executor_config = ZbobrExecutorConfig {
            mcp_tester: mcp_tester_config,
            ..Default::default()
        };

        let prompts = Prompts {
            base_path: None,
            preparator: vec![],
            planner: vec![],
            worker: vec![],
            reviewer: vec![],
            merger: vec![],
        };

        let task = self.get_task(task_id).await;

        process_task_by_stage(&self.zbobr, &task, None, &prompts, &executor_config)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] process_task_by_stage failed for task #{task_id}: {e}",
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
