use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::OnceCell;
use zbobr_dispatcher::Stage;

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
    /// Root directory for this integration-test run.  All tasks, workspaces,
    /// scenarios and scratch repos live under here.
    pub base_path: PathBuf,
    pub tasks_dir: PathBuf,
    pub workspaces_dir: PathBuf,
}

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------

static SHARED_ENV: OnceCell<Option<Arc<IntegrationTestEnv>>> = OnceCell::const_new();

impl IntegrationTestEnv {
    /// Return the shared environment, initialising it on the first call.
    ///
    /// Returns `None` when `mcp-tester` is not installed — callers should
    /// skip gracefully in that case.
    pub async fn get() -> Option<Arc<IntegrationTestEnv>> {
        SHARED_ENV
            .get_or_init(|| async { IntegrationTestEnv::init().await })
            .await
            .clone()
    }

    /// One-time setup: verify prerequisites, create directories, run
    /// `zbobr setup`.
    async fn init() -> Option<Arc<IntegrationTestEnv>> {
        let mcp_check = tokio::process::Command::new("mcp-tester")
            .arg("--version")
            .output()
            .await;
        if mcp_check.is_err() || !mcp_check.unwrap().status.success() {
            eprintln!("Skipping test: mcp-tester not installed (cargo install mcp-tester)");
            return None;
        }

        // Use CARGO_TARGET_TMPDIR so the directory persists for post-failure
        // analysis and is co-located with the test binary.
        let base_path = match std::env::var("CARGO_TARGET_TMPDIR") {
            Ok(p) => PathBuf::from(p).join("integration_env"),
            Err(_) => std::env::temp_dir().join("zbobr_integration_env"),
        };

        let tasks_dir = base_path.join("tasks");
        let workspaces_dir = base_path.join("workspaces");

        eprintln!(
            "[IntegrationTestEnv] base path: {}",
            base_path.display()
        );

        tokio::fs::create_dir_all(&base_path)
            .await
            .expect("failed to create integration env base dir");

        run_zbobr_impl(&base_path, &tasks_dir, &workspaces_dir, "setup", &[]).await;

        Some(Arc::new(IntegrationTestEnv {
            base_path,
            tasks_dir,
            workspaces_dir,
        }))
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
    pub async fn show_task(&self, task_id: u64) -> String {
        self.run_zbobr_capture("task", &["show", &task_id.to_string()])
            .await
    }

    // -----------------------------------------------------------------------
    // Git utilities
    // -----------------------------------------------------------------------

    /// Create a minimal git repository inside the environment's base directory,
    /// in a subdirectory named `<name>`.  Returns the absolute path to the
    /// newly-created repository.
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
            "[IntegrationTestEnv] git repo created: {}",
            repo_dir.display()
        );
        repo_dir
    }

    // -----------------------------------------------------------------------
    // zbobr CLI wrappers
    // -----------------------------------------------------------------------

    /// Execute `zbobr <command> [args…]` inheriting stdio.
    pub async fn run_zbobr(&self, command: &str, args: &[&str]) {
        run_zbobr_impl(&self.base_path, &self.tasks_dir, &self.workspaces_dir, command, args).await;
    }

    /// Execute `zbobr <command> [args…]` and return captured stdout.
    pub async fn run_zbobr_capture(&self, command: &str, args: &[&str]) -> String {
        run_zbobr_capture_impl(
            &self.base_path,
            &self.tasks_dir,
            &self.workspaces_dir,
            command,
            args,
        )
        .await
    }
}

// ---------------------------------------------------------------------------
// Low-level helpers (private, called only by IntegrationTestEnv methods)
// ---------------------------------------------------------------------------

fn make_zbobr_config_args(tasks_dir: &Path, workspaces_dir: &Path) -> Vec<String> {
    let mut args = Vec::new();
    let mut push = |flag: &str, val: &str| {
        args.push(flag.to_string());
        args.push(val.to_string());
    };

    push("--dispatcher-workspaces", &workspaces_dir.to_string_lossy());
    push("--tasks-fs-tasks-dir", &tasks_dir.to_string_lossy());
    push("--dispatcher-backend", "filesystem");
    push("--dispatcher-cli-tool", "mcp-tester");
    push("--dispatcher-agent-github-token", "dummy-not-used");
    push("--dispatcher-git-user-name", "test-bot");
    push("--dispatcher-git-user-email", "test@example.com");

    args
}

async fn run_zbobr_impl(
    base_path: &Path,
    tasks_dir: &Path,
    workspaces_dir: &Path,
    command: &str,
    command_args: &[&str],
) {
    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let mut args = make_zbobr_config_args(tasks_dir, workspaces_dir);
    args.push(command.to_string());
    // convert the slice of &str to owned Strings and extend the argument list
    args.extend(command_args.iter().map(|s| s.to_string()));

    let status = tokio::process::Command::new(zbobr_bin)
        .args(&args)
        .current_dir(base_path)
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
    base_path: &Path,
    tasks_dir: &Path,
    workspaces_dir: &Path,
    command: &str,
    command_args: &[&str],
) -> String {
    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let mut args = make_zbobr_config_args(tasks_dir, workspaces_dir);
    args.push(command.to_string());
    args.extend(command_args.iter().map(|s| s.to_string()));

    let output = tokio::process::Command::new(zbobr_bin)
        .args(&args)
        .current_dir(base_path)
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
