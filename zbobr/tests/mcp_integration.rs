use std::path::Path;
use tempfile::TempDir;
use zbobr_dispatcher::mcp::preparator_tools::{
    GET_DESCRIPTION, GET_DISCUSSION, SET_PARAM_DESTINATION_REPOSITORY,
    GET_PARAM_DESTINATION_REPOSITORY, SET_PARAM_DESTINATION_BRANCH,
    GET_PARAM_DESTINATION_BRANCH, SET_PARAM_WORK_BRANCH_POSTFIX, GET_PARAM_WORK_BRANCH,
};

/// Inline scenario YAML for simple testing (dummy get_description test)
fn dummy_scenario() -> String {
    format!(
        r#"name: Dummy MCP Test
description: Verify get_description returns the expected task description
timeout: 30
stop_on_failure: true

steps:
  - name: Get task description
    operation:
      type: tool_call
      tool: {GET_DESCRIPTION}
    assertions:
      - type: success
      - type: contains
        path: result
        value: "Dummy task description"
"#,
    )
}

/// Inline scenario YAML for comprehensive preparator testing
fn preparator_comprehensive_scenario() -> String {
    format!(
        r#"name: Preparator Comprehensive Test
description: Verify all PREPARATION MCP functions
timeout: 60
stop_on_failure: true

steps:
  - name: Get task description
    operation:
      type: tool_call
      tool: {GET_DESCRIPTION}
    assertions:
      - type: success
      - type: contains
        path: result
        value: "Dummy task description"

  - name: Get task discussion
    operation:
      type: tool_call
      tool: {GET_DISCUSSION}
    assertions:
      - type: success
      - type: contains
        path: result
        value: "No messages yet."

  - name: Set destination repository
    operation:
      type: tool_call
      tool: {SET_PARAM_DESTINATION_REPOSITORY}
      arguments:
        value: "owner/repo"
    assertions:
      - type: success

  - name: Get destination repository
    operation:
      type: tool_call
      tool: {GET_PARAM_DESTINATION_REPOSITORY}
    assertions:
      - type: success
      - type: equals
        path: result
        value: "owner/repo"

  - name: Set destination branch
    operation:
      type: tool_call
      tool: {SET_PARAM_DESTINATION_BRANCH}
      arguments:
        value: "main"
    assertions:
      - type: success

  - name: Get destination branch
    operation:
      type: tool_call
      tool: {GET_PARAM_DESTINATION_BRANCH}
    assertions:
      - type: success
      - type: equals
        path: result
        value: "main"

  - name: Set work branch
    operation:
      type: tool_call
      tool: {SET_PARAM_WORK_BRANCH_POSTFIX}
      arguments:
        value: "test"
    assertions:
      - type: success

  - name: Get work branch
    operation:
      type: tool_call
      tool: {GET_PARAM_WORK_BRANCH}
    assertions:
      - type: success
      - type: contains
        path: result
        value: "test"
"#,
    )
}

/// Minimal TOML configuration for testing
fn create_test_config(tasks_dir: &Path, workspaces_dir: &Path) -> String {
    format!(
        r#"[dispatcher]
cli_tool = "mcp-tester"
workspaces = "{workspaces}"
backend = "fs"
agent_github_token = "dummy-not-used"
git_user_name = "test-bot"
git_user_email = "test@example.com"

[dispatcher.prompts]
preparator = []
planner = []
worker = []
reviewer = []
merger = []

[task.fs]
tasks_dir = "{tasks}"

[repo.fs]
repos_dir = "./repos"
"#,
        tasks = tasks_dir.display(),
        workspaces = workspaces_dir.display(),
    )
}

async fn run_mcp_test(command: &str) {
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

    // Create minimal configuration file
    let config_path = tmp_path.join("zbobr.toml");
    let config_content = create_test_config(&tasks_dir, &workspaces_dir);
    tokio::fs::write(&config_path, config_content)
        .await
        .expect("failed to write config file");

    // Create task using the filesystem backend via a synchronous call
    // (We need to create it synchronously before spawning the async process)
    let task_id = {
        use zbobr_task_backend_fs::FilesystemTaskBackend;
        use zbobr_dispatcher::backend::TaskBackend;
        use zbobr_dispatcher::Stage;
        use std::collections::HashMap;

        let backend = FilesystemTaskBackend::new(None, None, &tasks_dir)
            .expect("failed to create task backend");
        
        tokio::runtime::Handle::current()
            .block_on(async {
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
            })
    };

    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");

    // Build command-line arguments
    let mut args = vec![
        command.to_string(),
        task_id.to_string(),
        "--config".to_string(),
        config_path.to_string_lossy().to_string(),
    ];

    // Add executor scenario file paths - map roles to scenario files
    let (prep_scenario, planning_scenario, working_scenario, reviewing_scenario, merging_scenario) =
        match command {
            "prepare" => (preparator_path.clone(), dummy_path.clone(), dummy_path.clone(), dummy_path.clone(), dummy_path.clone()),
            _ => (preparator_path.clone(), dummy_path.clone(), dummy_path.clone(), dummy_path.clone(), dummy_path.clone()),
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


