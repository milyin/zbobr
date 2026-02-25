use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::OnceCell;
use zbobr_dispatcher::Stage;

use super::github_config::GitHubTestConfig;

// ---------------------------------------------------------------------------
// Backend configuration
// ---------------------------------------------------------------------------

/// CLI argument bundle that differs between filesystem and GitHub backends.
enum BackendArgs {
    Filesystem {
        tasks_dir: PathBuf,
    },
    GitHub {
        task_repo: String,
        task_token: String,
        fork_owner: String,
        repo_token: String,
        agent_token: String,
    },
}

impl BackendArgs {
    fn name(&self) -> &'static str {
        match self {
            BackendArgs::Filesystem { .. } => "filesystem",
            BackendArgs::GitHub { .. } => "github",
        }
    }
}

// ---------------------------------------------------------------------------
// Shared environment
// ---------------------------------------------------------------------------

/// All shared paths and state that survive for the entire integration-test
/// binary run.  Created once via [`IntegrationTestEnv::get`] and then
/// re-used by every test function.
///
/// The environment is stored in the Cargo-managed test tmp directory
/// (`CARGO_TARGET_TMPDIR`) so that it is persisted (not cleaned up) after the
/// run, making post-failure analysis possible.  The full path is logged during
/// initialisation.
///
/// **Important:** this integration test uses *only* the command‑line
/// interface to create and manipulate tasks.  It must not instantiate or
/// call backend implementations directly so that the same test works with any
/// backend (filesystem, GitHub, etc.).
pub struct IntegrationTestEnv {
    /// Root directory for this integration-test run.  All workspaces,
    /// scenarios and scratch repos live under here.
    pub base_path: PathBuf,
    pub workspaces_dir: PathBuf,
    backend: BackendArgs,
}

// ---------------------------------------------------------------------------
// Singletons
// ---------------------------------------------------------------------------

static FS_ENV: OnceCell<Option<Arc<IntegrationTestEnv>>> = OnceCell::const_new();
static GITHUB_ENV: OnceCell<Option<Arc<IntegrationTestEnv>>> = OnceCell::const_new();

impl IntegrationTestEnv {
    /// Return the filesystem backend environment, initialising it on first call.
    ///
    /// Returns `None` when `mcp-tester` is not installed — callers should skip
    /// gracefully in that case.
    #[allow(dead_code)]
    pub async fn get() -> Option<Arc<IntegrationTestEnv>> {
        FS_ENV
            .get_or_init(|| async { IntegrationTestEnv::init_fs().await })
            .await
            .clone()
    }

    /// Return all available environments (filesystem always, GitHub when
    /// `zbobr_github_test.toml` is present at the workspace root).
    ///
    /// Returns an empty `Vec` when `mcp-tester` is not installed.
    pub async fn get_all() -> Vec<Arc<IntegrationTestEnv>> {
        let fs = FS_ENV
            .get_or_init(|| async { IntegrationTestEnv::init_fs().await })
            .await
            .clone();
        let github = GITHUB_ENV
            .get_or_init(|| async { IntegrationTestEnv::init_github().await })
            .await
            .clone();

        [fs, github].into_iter().flatten().collect()
    }

    // -----------------------------------------------------------------------
    // Initialisation
    // -----------------------------------------------------------------------

    /// Verify that `mcp-tester` is installed and return `true` if it is.
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

    /// Compute the base path for an environment identified by `suffix`.
    async fn make_base_path(suffix: &str) -> PathBuf {
        let base = match std::env::var("CARGO_TARGET_TMPDIR") {
            Ok(p) => PathBuf::from(p).join(format!("integration_env_{suffix}")),
            Err(_) => std::env::temp_dir().join(format!("zbobr_integration_env_{suffix}")),
        };
        // Remove leftover state from prior runs for a clean environment.
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

    /// One-time setup for the **filesystem** backend environment.
    async fn init_fs() -> Option<Arc<IntegrationTestEnv>> {
        if !Self::check_mcp_tester().await {
            return None;
        }

        let base_path = Self::make_base_path("fs").await;
        let tasks_dir = base_path.join("tasks");
        let workspaces_dir = base_path.join("workspaces");

        eprintln!("[IntegrationTestEnv/fs] base path: {}", base_path.display());

        let backend = BackendArgs::Filesystem {
            tasks_dir: tasks_dir.clone(),
        };
        let env = Arc::new(IntegrationTestEnv {
            base_path: base_path.clone(),
            workspaces_dir,
            backend,
        });

        run_zbobr_impl(&env, "setup", &[]).await;

        Some(env)
    }

    /// One-time setup for the **GitHub** backend environment.
    ///
    /// Returns `None` when `zbobr_github_test.toml` is absent or when the
    /// GitHub connectivity check fails.
    async fn init_github() -> Option<Arc<IntegrationTestEnv>> {
        if !Self::check_mcp_tester().await {
            return None;
        }

        let config = GitHubTestConfig::load()?;

        let base_path = Self::make_base_path("github").await;
        let workspaces_dir = base_path.join("workspaces");

        eprintln!(
            "[IntegrationTestEnv/github] base path: {}",
            base_path.display()
        );

        let backend = BackendArgs::GitHub {
            task_repo: config.tasks.github.task_repo,
            task_token: config.tasks.github.token,
            fork_owner: config.repo.github.fork_owner,
            repo_token: config.repo.github.token,
            agent_token: config.dispatcher.agent_token,
        };
        let env = Arc::new(IntegrationTestEnv {
            base_path: base_path.clone(),
            workspaces_dir,
            backend,
        });

        // Attempt setup; skip gracefully on failure (e.g. invalid token).
        let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");
        let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
        let mut args = zbobr_config_args(&env.backend, &env.workspaces_dir);
        args.push("setup".to_string());

        let status = tokio::process::Command::new(zbobr_bin)
            .args(&args)
            .current_dir(&base_path)
            .env("RUST_LOG", &rust_log)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .await
            .ok()?;

        if !status.success() {
            eprintln!(
                "[IntegrationTestEnv/github] setup failed — skipping GitHub integration tests. \
                 Check that zbobr_github_test.toml contains valid credentials."
            );
            return None;
        }

        Some(env)
    }

    // -----------------------------------------------------------------------
    // Identification
    // -----------------------------------------------------------------------

    /// Human-readable name of the active backend ("filesystem" or "github").
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    // -----------------------------------------------------------------------
    // Task utilities
    // -----------------------------------------------------------------------

    /// Create a new task via the zbobr CLI and return the assigned task ID.
    pub async fn create_task(&self, title: &str, description: &str, stage: Stage) -> u64 {
        let stage_str = stage.to_string();
        let output = self
            .run_zbobr_capture(
                "task",
                &[
                    "create",
                    title,
                    "--description",
                    description,
                    "--stage",
                    &stage_str,
                ],
            )
            .await;

        let line = output
            .lines()
            .find(|l| l.trim().starts_with("Created task #"))
            .unwrap_or_default();
        line.trim()
            .strip_prefix("Created task #")
            .and_then(|s| s.parse::<u64>().ok())
            .expect("failed to parse task id from zbobr output")
    }

    /// Show a task and return its raw text output.
    #[allow(dead_code)]
    pub async fn show_task(&self, task_id: u64) -> String {
        self.run_zbobr_capture("task", &["show", &task_id.to_string()])
            .await
    }

    /// Process a task according to its current stage.
    #[allow(dead_code)]
    pub async fn process_task(&self, task_id: u64) {
        let task_id_str = task_id.to_string();
        self.run_zbobr("task", &["process", &task_id_str]).await;
    }

    /// Update a task's repository and branch information.
    #[allow(dead_code)]
    pub async fn update_task_branches(
        &self,
        task_id: u64,
        dest_repo: &str,
        dest_branch: &str,
        work_branch: &str,
    ) {
        let task_id_str = task_id.to_string();
        self.run_zbobr(
            "task",
            &[
                "update",
                &task_id_str,
                "--dest-repo",
                dest_repo,
                "--dest-branch",
                dest_branch,
                "--work-branch",
                work_branch,
            ],
        )
        .await;
    }

    /// Read a task's current stage from CLI output.
    #[allow(dead_code)]
    pub async fn task_stage(&self, task_id: u64) -> Stage {
        let output = self.show_task(task_id).await;
        let stage_line = output
            .lines()
            .find(|l| l.trim_start().starts_with("Stage:"))
            .unwrap_or_else(|| panic!("Stage line not found in output: {output}"));
        let stage_value = stage_line
            .split(':')
            .nth(1)
            .map(str::trim)
            .unwrap_or_else(|| panic!("Malformed stage line: {stage_line}"));

        Stage::from_milestone_name(stage_value)
            .unwrap_or_else(|| panic!("Unknown stage '{stage_value}' in output: {output}"))
    }

    // -----------------------------------------------------------------------
    // Git utilities
    // -----------------------------------------------------------------------

    /// Create a minimal git repository inside the environment's base directory,
    /// in a subdirectory named `<name>`.  Returns the absolute path to the
    /// newly-created repository.
    ///
    /// For both the filesystem and GitHub backends the local repo is used as
    /// the code workspace; the tests exercise MCP tool behaviour (task storage)
    /// rather than the actual git/GitHub repo backend.
    pub async fn create_git_repo(&self, name: &str) -> PathBuf {
        let repo_dir = self.base_path.join(name);
        tokio::fs::create_dir_all(&repo_dir).await.unwrap();

        async fn git(repo_dir: &PathBuf, args: &[&str]) {
            let status = tokio::process::Command::new("git")
                .args(args)
                .current_dir(repo_dir)
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

        eprintln!(
            "[IntegrationTestEnv/{}] git repo created: {}",
            self.backend_name(),
            repo_dir.display()
        );
        repo_dir
    }

    // -----------------------------------------------------------------------
    // zbobr CLI wrappers
    // -----------------------------------------------------------------------

    /// Execute `zbobr <command> [args…]` inheriting stdio.
    pub async fn run_zbobr(&self, command: &str, args: &[&str]) {
        run_zbobr_impl(self, command, args).await;
    }

    /// Execute `zbobr <command> [args…]` and return captured stdout.
    pub async fn run_zbobr_capture(&self, command: &str, args: &[&str]) -> String {
        run_zbobr_capture_impl(self, command, args).await
    }

    /// Prepare a workspace directory for a task by cloning the source repo
    /// and setting up the work branch. This simulates what the dispatcher
    /// would do before starting an agent session.
    #[allow(dead_code)]
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

        // Clone the source repo into the workspace.
        let clone_status = tokio::process::Command::new("git")
            .args([
                "clone",
                repo_path.to_str().unwrap(),
                work_dir.to_str().unwrap(),
            ])
            .status()
            .await
            .expect("failed to run git clone");
        assert!(clone_status.success(), "git clone failed");

        // Try to checkout an existing work branch, or create a new one from HEAD.
        let checkout = tokio::process::Command::new("git")
            .args(["checkout", work_branch])
            .current_dir(&work_dir)
            .output()
            .await
            .expect("failed to run git checkout");

        if !checkout.status.success() {
            let create = tokio::process::Command::new("git")
                .args(["checkout", "-b", work_branch])
                .current_dir(&work_dir)
                .status()
                .await
                .expect("failed to run git checkout -b");
            assert!(
                create.success(),
                "failed to create work branch '{work_branch}'"
            );
        }

        work_dir
    }

    /// Create a scenario file and return its path.
    #[allow(dead_code)]
    pub async fn create_scenario(&self, name: &str, content: &str) -> String {
        let scenarios_dir = self.base_path.join("scenarios");
        tokio::fs::create_dir_all(&scenarios_dir)
            .await
            .expect("failed to create scenarios directory");
        let scenario_path = scenarios_dir.join(format!("{name}.yml"));
        tokio::fs::write(&scenario_path, content)
            .await
            .expect("failed to write scenario");
        scenario_path.to_string_lossy().to_string()
    }
}

// ---------------------------------------------------------------------------
// Low-level helpers (private)
// ---------------------------------------------------------------------------

fn zbobr_config_args(backend: &BackendArgs, workspaces_dir: &Path) -> Vec<String> {
    let mut args = Vec::new();
    let mut push = |flag: &str, val: &str| {
        args.push(flag.to_string());
        args.push(val.to_string());
    };

    push("--dispatcher-workspaces", &workspaces_dir.to_string_lossy());
    push("--dispatcher-cli-tool", "mcp-tester");
    push("--dispatcher-git-user-name", "test-bot");
    push("--dispatcher-git-user-email", "test@example.com");

    match backend {
        BackendArgs::Filesystem { tasks_dir } => {
            push("--dispatcher-backend", "filesystem");
            push("--tasks-fs-tasks-dir", &tasks_dir.to_string_lossy());
            push("--dispatcher-agent-github-token", "dummy-not-used");
        }
        BackendArgs::GitHub {
            task_repo,
            task_token,
            fork_owner,
            repo_token,
            agent_token,
        } => {
            push("--dispatcher-backend", "github");
            push("--tasks-github-task-repo", task_repo);
            push("--tasks-github-token", task_token);
            push("--repo-github-fork-owner", fork_owner);
            push("--repo-github-token", repo_token);
            push("--dispatcher-agent-github-token", agent_token);
        }
    }

    args
}

async fn run_zbobr_impl(env: &IntegrationTestEnv, command: &str, command_args: &[&str]) {
    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let mut args = zbobr_config_args(&env.backend, &env.workspaces_dir);
    args.push(command.to_string());
    args.extend(command_args.iter().map(|s| s.to_string()));

    let status = tokio::process::Command::new(zbobr_bin)
        .args(&args)
        .current_dir(&env.base_path)
        .env("RUST_LOG", &rust_log)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .expect("failed to run zbobr");

    assert!(
        status.success(),
        "zbobr {} failed with exit code {:?}",
        command,
        status.code(),
    );
}

async fn run_zbobr_capture_impl(
    env: &IntegrationTestEnv,
    command: &str,
    command_args: &[&str],
) -> String {
    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let mut args = zbobr_config_args(&env.backend, &env.workspaces_dir);
    args.push(command.to_string());
    args.extend(command_args.iter().map(|s| s.to_string()));

    let output = tokio::process::Command::new(zbobr_bin)
        .args(&args)
        .current_dir(&env.base_path)
        .env("RUST_LOG", &rust_log)
        .output()
        .await
        .expect("failed to run zbobr");

    assert!(
        output.status.success(),
        "zbobr {} failed with exit code {:?}",
        command,
        output.status.code(),
    );

    String::from_utf8_lossy(&output.stdout).to_string()
}
