use std::collections::HashMap;
use std::sync::Arc;

use tempfile::TempDir;
use zbobr_dispatcher::backend::{RepoBackend, TaskBackend};
use zbobr_dispatcher::config::BackendType;
use zbobr_dispatcher::task::{Model, Role, Tool};
use zbobr_dispatcher::{Stage, ToolExecutor, Zbobr, ZbobrDispatcherConfig};
use zbobr_executor_mcp_tester::{McpTesterExecutor, ZbobrExecutorMcpTesterConfig};
use zbobr_repo_backend_fs::FilesystemRepoBackend;
use zbobr_task_backend_fs::FilesystemTaskBackend;

#[tokio::test]
async fn planning_get_description_via_mcp_tester() {
    // Check that mcp-tester is installed; skip gracefully if not
    let mcp_check = tokio::process::Command::new("mcp-tester")
        .arg("--version")
        .output()
        .await;
    if mcp_check.is_err() || !mcp_check.unwrap().status.success() {
        eprintln!("Skipping test: mcp-tester not installed (cargo install mcp-tester)");
        return;
    }

    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init();

    // Set up temp directories
    let tmp = TempDir::new().expect("failed to create temp dir");
    let tasks_dir = tmp.path().join("tasks");
    let repos_dir = tmp.path().join("repos");
    let workspaces_dir = tmp.path().join("workspaces");

    // Create FS backends
    let task_backend: Arc<dyn TaskBackend> = Arc::new(
        FilesystemTaskBackend::new(None, Some(tasks_dir.to_str().unwrap()))
            .expect("failed to create task backend"),
    );
    let repo_backend: Arc<dyn RepoBackend> = Arc::new(
        FilesystemRepoBackend::new(None, Some(repos_dir.to_str().unwrap()))
            .expect("failed to create repo backend"),
    );

    // Create a task with a known description in GoPlanning stage
    let expected_description = "Implement the frobnicator module for the bar subsystem";
    let task_id = task_backend
        .create_task(
            "Test Planning Task",
            expected_description,
            Stage::GoPlanning,
            Some(Tool::McpTester),
            None,
            HashMap::new(),
        )
        .await
        .expect("failed to create task");

    // Build dispatcher config
    let config = ZbobrDispatcherConfig {
        default_model: Model::default(),
        workspaces: workspaces_dir.clone(),
        agent_github_token: String::new(),
        copilot_github_token: String::new(),
        backend: BackendType::default(),
        cli_tool: Tool::McpTester,
        planner_prompts: vec![],
        worker_prompts: vec![],
        reviewer_prompts: vec![],
        merger_prompts: vec![],
        work_branch_prefix: "zbobr_fix".to_string(),
        prompts_path: None,
        git_user_name: String::new(),
        git_user_email: String::new(),
    };

    let zbobr = Zbobr::new(config, task_backend, repo_backend);

    // Start MCP server for planner role
    let port = zbobr_dispatcher::mcp::run_role_mcp_server(
        zbobr.clone(),
        19000,
        Role::Planner,
        task_id,
    )
    .await
    .expect("failed to start MCP server");

    // Give the server a moment to start accepting connections
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Write planning.yml scenario file
    let scenario_content = format!(
        r#"name: Planning Description Test
description: Verify get_description returns the expected task description
timeout: 30
stop_on_failure: true

steps:
  - name: Get task description
    operation:
      type: tool_call
      tool: get_description
    assertions:
      - type: success
      - type: contains
        path: result
        value: "{}"
"#,
        expected_description
    );

    let scenario_path = tmp.path().join("planning.yml");
    tokio::fs::write(&scenario_path, &scenario_content)
        .await
        .expect("failed to write scenario file");

    // Create task workspace directory (executor uses it as cwd)
    let task_dir = workspaces_dir.join(format!("task#{}", task_id));
    tokio::fs::create_dir_all(&task_dir)
        .await
        .expect("failed to create task dir");

    // Create executor config pointing to the scenario file
    let mcp_tester_config = ZbobrExecutorMcpTesterConfig {
        planning: Some(scenario_path),
        ..Default::default()
    };

    let executor = McpTesterExecutor {
        config: mcp_tester_config,
    };

    let mcp_url = format!("http://127.0.0.1:{}/planner/{}", port, task_id);

    let result = executor
        .execute(
            task_id,
            Role::Planner,
            &Model::default(),
            port,
            "",
            &task_dir,
            &mcp_url,
            "",
            "",
        )
        .await;

    assert!(
        result.is_ok(),
        "mcp-tester execution failed: {:?}",
        result.err()
    );
}
