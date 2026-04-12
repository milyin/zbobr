#![allow(dead_code)]

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use indexmap::IndexMap;
use zbobr_api::{
    Model, Secret,
    config::{Provider, ProviderDefinition, Tool, ToolEntry, WorkflowConfig},
};
use zbobr_dispatcher::{
    Comment, Task, Workflow, ZbobrDispatcher, ZbobrDispatcherBuilder, ZbobrDispatcherConfig,
    backend::TaskBackendExt, prompts::ConfiguredPromptBuilder, task::Executor,
};
use zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterConfig;
use zbobr_repo_backend_fs::{ZbobrRepoBackendFs, ZbobrRepoBackendFsConfig};
use zbobr_repo_backend_github::{ZbobrRepoBackendGithub, ZbobrRepoBackendGithubConfig};
use zbobr_task_backend_fs::{ArcTaskBackendFs, ZbobrTaskBackendFs, ZbobrTaskBackendFsConfig};
use zbobr_task_backend_github::{TaskBackendGithub, ZbobrTaskBackendGithubConfig};

static SCENARIO_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Build the standard test provider/tool entries for integration tests.
///
/// Defines one provider "mcp-tester" backed by the mcp-tester executor, and one
/// tool also named "mcp-tester" that selects that provider with model "test-model".
fn test_providers_and_tools() -> (
    IndexMap<Provider, ProviderDefinition>,
    IndexMap<Tool, Vec<ToolEntry>>,
) {
    let provider = ProviderDefinition {
        executor: Some(Executor(Executor::MCP_TESTER.to_string())).into(),
        parent: Default::default(),
        priority: Default::default(),
        plan_mode: Default::default(),
        access_key: Default::default(),
    };
    let entry = ToolEntry {
        provider: "mcp-tester".to_string().into(),
        model: Model::try_new("test-model").expect("valid model name"),
        priority: Default::default(),
    };
    let providers = IndexMap::from([("mcp-tester".to_string().into(), provider)]);
    let tools = IndexMap::from([("mcp-tester".to_string().into(), vec![entry])]);
    (providers, tools)
}

/// Shared environment for integration tests.  Holds a live `ZbobrDispatcher`
/// and backends — no CLI binary involved.
pub struct IntegrationTestEnv {
    pub base_path: PathBuf,
    pub workspaces_dir: PathBuf,
    pub name: &'static str,
    pub local_repo_path: Option<PathBuf>,
    /// Default dispatcher (with default workflow) for convenience
    /// methods like `create_task`, `get_task`, `setup_repository`, etc.
    pub zbobr: Arc<ZbobrDispatcher>,
    /// Factory to create a dispatcher with a specific workflow for `process_task`.
    dispatcher_factory: Box<dyn Fn(Workflow) -> Arc<ZbobrDispatcher> + Send + Sync>,
    /// Optional remote repository slug (`owner/repo`) used by GitHub repo-backend tests.
    /// `None` for the filesystem repo backend.
    pub target_repo: Option<String>,
}

/// Construct an environment backed by two filesystem backends.
///
/// Returns `None` when `mcp-tester` is not installed (tests are skipped).
pub async fn init_fs_fs(name: &'static str) -> Option<Arc<IntegrationTestEnv>> {
    if !check_mcp_tester().await {
        return None;
    }

    let base_path = make_base_path(name).await;

    eprintln!(
        "[IntegrationTestEnv/{name}] base path: {}",
        base_path.display()
    );

    let (providers, tools) = test_providers_and_tools();
    let mut dispatcher_config = ZbobrDispatcherConfig {
        workspaces: base_path.join("workspaces"),
        providers,
        tools,
        git_user_name: "test-bot".to_string(),
        git_user_email: "test@example.com".to_string(),
        overwrite_author: false,
        ..ZbobrDispatcherConfig::default()
    };
    dispatcher_config
        .agent_github_token
        .resolve()
        .expect("agent_github_token");

    // Append instance name to workspaces and repos_dir for directory isolation.
    dispatcher_config.workspaces = dispatcher_config
        .workspaces
        .join(&dispatcher_config.instance);
    let workspaces_dir = dispatcher_config.workspaces.clone();

    // Create a shared test git repository for the FS repo backend.
    let test_repo_dir = base_path.join("test_repo");
    tokio::fs::create_dir_all(&test_repo_dir).await.ok()?;
    for args in [
        vec!["init"],
        vec!["config", "user.name", "test-bot"],
        vec!["config", "user.email", "test@example.com"],
    ] {
        tokio::process::Command::new("git")
            .args(&args)
            .current_dir(&test_repo_dir)
            .status()
            .await
            .ok()?
            .success()
            .then_some(())?;
    }
    tokio::fs::write(test_repo_dir.join("README.md"), "test repo")
        .await
        .ok()?;
    for args in [
        vec!["add", "README.md"],
        vec!["commit", "-m", "Initial commit"],
        vec!["branch", "-M", "main"],
    ] {
        tokio::process::Command::new("git")
            .args(&args)
            .current_dir(&test_repo_dir)
            .status()
            .await
            .ok()?
            .success()
            .then_some(())?;
    }

    let task_backend_config = ZbobrTaskBackendFsConfig {
        tasks_dir: base_path.join("tasks"),
        default_max_stage_count: zbobr_api::task::DEFAULT_MAX_STAGE_COUNT,
        timezone: None,
    };
    let repo_backend_config = ZbobrRepoBackendFsConfig {
        repository: test_repo_dir.to_string_lossy().to_string(),
        branch: "main".to_string(),
        repos_dir: base_path.join("repos").join(&dispatcher_config.instance),
    };

    let task_backend =
        ArcTaskBackendFs::new(ZbobrTaskBackendFs::from_config(task_backend_config.clone()).ok()?);
    let repo_backend = ZbobrRepoBackendFs::from_config(repo_backend_config.clone()).ok()?;

    let default_workflow = Workflow::default();
    let zbobr = Arc::new(
        ZbobrDispatcherBuilder::new()
            .with_config(dispatcher_config.clone())
            .with_workflow(default_workflow.clone())
            .with_task_backend(task_backend)
            .with_repo_backend(repo_backend)
            .with_prompt_builder(ConfiguredPromptBuilder::new(
                None,
                Arc::new(default_workflow),
            ))
            .build()
            .validated()
            .expect("test dispatcher config must be valid"),
    );

    zbobr.setup_repository(false).await.ok()?;

    let factory_config = dispatcher_config;
    let factory_task_config = task_backend_config;
    let factory_repo_config = repo_backend_config;
    let dispatcher_factory = Box::new(move |workflow: Workflow| {
        let tb = ArcTaskBackendFs::new(
            ZbobrTaskBackendFs::from_config(factory_task_config.clone()).unwrap(),
        );
        let rb = ZbobrRepoBackendFs::from_config(factory_repo_config.clone()).unwrap();
        Arc::new(
            ZbobrDispatcherBuilder::new()
                .with_config(factory_config.clone())
                .with_workflow(workflow.clone())
                .with_task_backend(tb)
                .with_repo_backend(rb)
                .with_prompt_builder(ConfiguredPromptBuilder::new(None, Arc::new(workflow)))
                .build()
                .validated()
                .expect("test dispatcher config must be valid"),
        )
    });

    Some(Arc::new(IntegrationTestEnv {
        base_path,
        workspaces_dir,
        name,
        local_repo_path: Some(test_repo_dir),
        zbobr,
        dispatcher_factory,
        target_repo: None,
    }))
}

/// Construct an environment backed by GitHub task and repo backends.
///
/// Returns `None` when `mcp-tester` is not installed (tests are skipped).
pub async fn init_github_github(
    name: &'static str,
    task_repo: String,
    task_token: String,
    repository: String,
    branch: String,
    repo_token: String,
) -> Option<Arc<IntegrationTestEnv>> {
    install_rustls_provider();
    if !check_mcp_tester().await {
        return None;
    }

    let base_path = make_base_path(name).await;

    eprintln!(
        "[IntegrationTestEnv/{name}] base path: {}",
        base_path.display()
    );

    let (providers, tools) = test_providers_and_tools();
    let mut dispatcher_config = ZbobrDispatcherConfig {
        workspaces: base_path.join("workspaces"),
        providers,
        tools,
        ..ZbobrDispatcherConfig::default()
    };
    dispatcher_config
        .agent_github_token
        .resolve()
        .expect("agent_github_token");

    // Append instance name to workspaces and repos_dir for directory isolation.
    dispatcher_config.workspaces = dispatcher_config
        .workspaces
        .join(&dispatcher_config.instance);
    let workspaces_dir = dispatcher_config.workspaces.clone();

    let task_backend_config = ZbobrTaskBackendGithubConfig {
        instance: dispatcher_config.instance.clone(),
        timezone: None,
        github_repo: task_repo.clone(),
        github_token: Secret::value(task_token),
        reports_branch: None,
        reports_path: None,
        allowed_usernames: None,
        default_max_stage_count: zbobr_api::task::DEFAULT_MAX_STAGE_COUNT,
    };
    let repo_backend_config = ZbobrRepoBackendGithubConfig {
        repository: repository.clone(),
        branch: branch.clone(),
        github_token: Secret::value(repo_token),
        repos_dir: base_path.join("repos").join(&dispatcher_config.instance),
    };

    let task_backend = TaskBackendGithub::from_config(task_backend_config.clone()).ok()?;
    let repo_backend = ZbobrRepoBackendGithub::from_config(repo_backend_config.clone()).ok()?;

    let default_workflow = Workflow::default();
    let zbobr = Arc::new(
        ZbobrDispatcherBuilder::new()
            .with_config(dispatcher_config.clone())
            .with_workflow(default_workflow.clone())
            .with_task_backend(task_backend)
            .with_repo_backend(repo_backend)
            .with_prompt_builder(ConfiguredPromptBuilder::new(
                None,
                Arc::new(default_workflow),
            ))
            .build()
            .validated()
            .expect("test dispatcher config must be valid"),
    );

    zbobr.setup_repository(false).await.ok()?;

    let factory_config = dispatcher_config;
    let factory_task_config = task_backend_config;
    let factory_repo_config = repo_backend_config;
    let dispatcher_factory = Box::new(move |workflow: Workflow| {
        let tb = TaskBackendGithub::from_config(factory_task_config.clone()).unwrap();
        let rb = ZbobrRepoBackendGithub::from_config(factory_repo_config.clone()).unwrap();
        Arc::new(
            ZbobrDispatcherBuilder::new()
                .with_config(factory_config.clone())
                .with_workflow(workflow.clone())
                .with_task_backend(tb)
                .with_repo_backend(rb)
                .with_prompt_builder(ConfiguredPromptBuilder::new(None, Arc::new(workflow)))
                .build()
                .validated()
                .expect("test dispatcher config must be valid"),
        )
    });

    Some(Arc::new(IntegrationTestEnv {
        base_path,
        workspaces_dir,
        name,
        local_repo_path: None,
        zbobr,
        dispatcher_factory,
        target_repo: Some(repository),
    }))
}

impl IntegrationTestEnv {
    // -----------------------------------------------------------------------
    // Dispatcher factory
    // -----------------------------------------------------------------------

    /// Create a dispatcher with a specific workflow (for `process_task` calls).
    pub fn make_dispatcher(&self, workflow: WorkflowConfig) -> Arc<ZbobrDispatcher> {
        (self.dispatcher_factory)(Workflow::from_config(workflow))
    }

    // -----------------------------------------------------------------------
    // Identification
    // -----------------------------------------------------------------------

    pub fn name(&self) -> &'static str {
        self.name
    }

    // -----------------------------------------------------------------------
    // Task utilities
    // -----------------------------------------------------------------------

    pub async fn create_task(&self, title: &str, description: &str, state: &str) -> u64 {
        self.zbobr
            .create_task(title, description, state)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to create task: {e}", self.name))
    }

    pub async fn get_task(&self, task_id: u64) -> Task {
        self.zbobr
            .task_backend()
            .get_task(task_id)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to get task #{task_id}: {e}", self.name))
            .snapshot(false)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to snapshot task #{task_id}: {e}", self.name))
    }

    pub async fn get_comments(&self, task_id: u64) -> Vec<Comment> {
        self.zbobr
            .task_backend()
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

    pub async fn update_task_branches(&self, task_id: u64, work_branch: &str) {
        let work_branch = work_branch.to_string();
        let weak = self
            .zbobr
            .task_backend()
            .get_task(task_id)
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to get task #{task_id}: {e}", self.name));
        let mutable = weak
            .upgrade()
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to upgrade task #{task_id}: {e}", self.name));
        mutable
            .modify_task(Box::new(move |mut task| {
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
    // Pipeline runner
    // -----------------------------------------------------------------------

    /// Run the dispatcher with a full workflow config and per-role scenario map.
    ///
    /// Accepts an arbitrary `WorkflowConfig` with any pipeline/stage/role names.
    /// Scenarios are mapped via the generic `scenarios` HashMap in the mcp-tester config.
    pub async fn run_pipeline(
        &self,
        task_id: u64,
        workflow: &WorkflowConfig,
        role_scenarios: &HashMap<String, String>,
    ) {
        self.zbobr
            .task_backend()
            .set_task_state(task_id, zbobr_api::State::pending(workflow.default_pipeline()))
            .await
            .unwrap_or_else(|e| panic!("[{}] failed to set task state: {e}", self.name));
        self.zbobr
            .task_backend()
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
        };

        let task = self.get_task(task_id).await;
        let zbobr = self.make_dispatcher(workflow.clone());

        zbobr
            .process_task(&task, Some(&mcp_tester_config))
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
        workflow: &WorkflowConfig,
        role_scenarios: &HashMap<String, String>,
        max_iterations: usize,
    ) -> usize {
        let zbobr = self.make_dispatcher(workflow.clone());
        for i in 0..max_iterations {
            let task = self.get_task(task_id).await;
            if task.state.is_done() {
                return i;
            }
            if task.state.is_pause() {
                // Resume via advance_tasks, which runs the pending_override pre-pass.
                zbobr.advance_tasks().await.ok();
                let resumed_task = self.get_task(task_id).await;
                if resumed_task.state.is_pause() {
                    return i;
                }
                continue;
            }
            if task.go_pause {
                // advance_tasks converts the pause flag to PAUSE state via apply_pause_to_state.
                zbobr.advance_tasks().await.ok();
                return i;
            }
            // If state is PENDING but no signal, nothing to do
            if task.state.is_pending() && task.signal.is_none() {
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
            };

            let task = self.get_task(task_id).await;

            zbobr
                .process_task(&task, Some(&mcp_tester_config))
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
        workflow: &WorkflowConfig,
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
        };

        let task = self.get_task(task_id).await;
        let zbobr = self.make_dispatcher(workflow.clone());

        zbobr
            .process_task(&task, Some(&mcp_tester_config))
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] process_task (continue) failed for task #{task_id}: {e}",
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
