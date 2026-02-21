
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
        use zbobr_task_backend_fs::FilesystemTaskBackend;

        let backend = FilesystemTaskBackend::new(
            None,
            zbobr_task_backend_fs::ZbobrTaskBackendFsArgs::default(),
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

    // Build command-line arguments (ignore empty values)
    let mut args = vec![command.to_string(), task_id.to_string()];

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

    args.push("--executor-mcp-tester-preparation".to_string());
    args.push(prep_scenario.to_string_lossy().to_string());
    args.push("--executor-mcp-tester-planning".to_string());
    args.push(planning_scenario.to_string_lossy().to_string());
    args.push("--executor-mcp-tester-working".to_string());
    args.push(working_scenario.to_string_lossy().to_string());
    args.push("--executor-mcp-tester-reviewing".to_string());
    args.push(reviewing_scenario.to_string_lossy().to_string());
    args.push("--executor-mcp-tester-merging".to_string());
    args.push(merging_scenario.to_string_lossy().to_string());

    // Run zbobr binary
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

#[ignore = "Ignored during comand line refactoring to avoind intereference. To be re-enabled once CLI refactoring is complete."]
#[tokio::test]
async fn preparator_thorough_test_via_mcp_tester() {
    run_mcp_test("prepare").await;
}

#[ignore = "Ignored during comand line refactoring to avoind intereference. To be re-enabled once CLI refactoring is complete."]
#[tokio::test]
async fn planner_get_description_via_mcp_tester() {
    run_mcp_test("plan").await;
}

#[ignore = "Ignored during comand line refactoring to avoind intereference. To be re-enabled once CLI refactoring is complete."]
#[tokio::test]
async fn worker_get_description_via_mcp_tester() {
    run_mcp_test("work").await;
}

#[ignore = "Ignored during comand line refactoring to avoind intereference. To be re-enabled once CLI refactoring is complete."]
#[tokio::test]
async fn reviewer_get_description_via_mcp_tester() {
    run_mcp_test("review").await;
}

#[ignore = "Ignored during comand line refactoring to avoind intereference. To be re-enabled once CLI refactoring is complete."]
#[tokio::test]
async fn merger_get_description_via_mcp_tester() {
    run_mcp_test("merge").await;
}
