
use tempfile::TempDir;

mod mcp_tester_scenarios;
use mcp_tester_scenarios::{dummy_scenario, preparator_comprehensive_scenario};


async fn run_mcp_test(command: &str) {
    // Build and pass configuration via command-line flags rather than a TOML file.
    // Arguments with empty values are skipped so tests can selectively include flags.
    // Check that mcp-tester is installed; skip gracefully if not
    let mcp_check = tokio::process::Command::new("mcp-tester")
        .arg("--version")
        .output()
        .await;
    if mcp_check.is_err() || !mcp_check.unwrap().status.success() {
        eprintln!("Skipping test: mcp-tester not installed (cargo install mcp-tester)");
        return;
    }

    // Create temp directory for the entire test setup
    let tmp = TempDir::new().expect("failed to create temp dir");
    let tmp_path = tmp.path();


    // Create subdirectories
    let tasks_dir = tmp_path.join("tasks");
    let scenarios_dir = tmp_path.join("scenarios");
    let workspaces_dir = tmp_path.join("workspaces");

    tokio::fs::create_dir_all(&tasks_dir)
        .await
        .expect("failed to create tasks directory");
    tokio::fs::create_dir_all(&scenarios_dir)
        .await
        .expect("failed to create scenarios directory");
    tokio::fs::create_dir_all(&workspaces_dir)
        .await
        .expect("failed to create workspaces directory");

    // Write scenario files
    let dummy_path = scenarios_dir.join("dummy.yml");
    let preparator_path = scenarios_dir.join("preparator_comprehensive.yml");

    tokio::fs::write(&dummy_path, dummy_scenario())
        .await
        .expect("failed to write dummy scenario");
    tokio::fs::write(&preparator_path, preparator_comprehensive_scenario())
        .await
        .expect("failed to write preparator scenario");


    // Create task using the filesystem backend. we can await directly
    let task_id = {
        use std::collections::HashMap;

        use zbobr_dispatcher::{Stage, backend::TaskBackend};
        use zbobr_task_backend_fs::{FilesystemTaskBackend, ZbobrTaskBackendFsArgs};

        // pass the temporary directory explicitly so we don't pollute the repo
        let backend = FilesystemTaskBackend::new(
            None,
            ZbobrTaskBackendFsArgs {
                tasks_dir: Some(tasks_dir.clone()),
            },
            &tasks_dir,
        )
        .expect("failed to create task backend");

        backend
            .create_task(
                "Dummy Task",
                "Dummy task description",
                Stage::Preparation,
                None,
                None,
                HashMap::new(),
            )
            .await
            .expect("failed to create task")
    };

    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");

    // Build command-line arguments (ignore empty values).
    // Global options must appear *before* the subcommand; the task ID follows the
    // command itself. Executor flags are also global, so we add them early as
    // well.
    let mut args = Vec::new();

    // helper closure pushes flag+value only if value is non-empty
    let mut push_arg = |flag: &str, val: &str| {
        if !val.is_empty() {
            args.push(flag.to_string());
            args.push(val.to_string());
        }
    };

    push_arg("--dispatcher-workspaces", &workspaces_dir.to_string_lossy());
    push_arg("--tasks-fs-tasks-dir", &tasks_dir.to_string_lossy());
    push_arg("--dispatcher-backend", "filesystem");
    push_arg("--dispatcher-cli-tool", "mcp-tester");
    push_arg("--dispatcher-agent-github-token", "dummy-not-used");
    push_arg("--dispatcher-git-user-name", "test-bot");
    push_arg("--dispatcher-git-user-email", "test@example.com");

    // Add executor scenario file paths - map roles to scenario files
    let (prep_scenario, planning_scenario, working_scenario, reviewing_scenario, merging_scenario) =
        match command {
            "prepare" => (
                preparator_path.clone(),
                dummy_path.clone(),
                dummy_path.clone(),
                dummy_path.clone(),
                dummy_path.clone(),
            ),
            _ => (
                preparator_path.clone(),
                dummy_path.clone(),
                dummy_path.clone(),
                dummy_path.clone(),
                dummy_path.clone(),
            ),
        };

    // executor scenarios (also global)
    push_arg("--executor-mcp-tester-preparation", &prep_scenario.to_string_lossy());
    push_arg("--executor-mcp-tester-planning", &planning_scenario.to_string_lossy());
    push_arg("--executor-mcp-tester-working", &working_scenario.to_string_lossy());
    push_arg("--executor-mcp-tester-reviewing", &reviewing_scenario.to_string_lossy());
    push_arg("--executor-mcp-tester-merging", &merging_scenario.to_string_lossy());

    // finally add the command and task id
    args.push(command.to_string());
    args.push(task_id.to_string());


    // Run zbobr binary.  Recent CLI refactor removed the ability to override
    // paths via environment variables, so rely solely on the command-line
    // flags we already constructed.  The earlier part of this function uses a
    // temporary filesystem backend to create the task, so everything should stay
    // within `tmp_path` and the repo directory remains untouched.
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let status = tokio::process::Command::new(zbobr_bin)
        .args(&args)
        .current_dir(tmp_path)
        .env("RUST_LOG", &rust_log)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .expect("failed to run zbobr binary");


    assert!(
        status.success(),
        "zbobr {} failed with exit code {:?}",
        command,
        status.code(),
    );
}

#[tokio::test]
async fn preparator_thorough_test_via_mcp_tester() {
    run_mcp_test("prepare").await;
}

#[tokio::test]
async fn planner_get_description_via_mcp_tester() {
    run_mcp_test("plan").await;
}

#[tokio::test]
async fn worker_get_description_via_mcp_tester() {
    run_mcp_test("work").await;
}

#[tokio::test]
async fn reviewer_get_description_via_mcp_tester() {
    run_mcp_test("review").await;
}

#[tokio::test]
async fn merger_get_description_via_mcp_tester() {
    run_mcp_test("merge").await;
}
