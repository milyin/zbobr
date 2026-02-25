mod mcp_integration;

use mcp_integration::IntegrationTestEnv;
use zbobr_dispatcher::Stage;

/// Planner scenario that exercises the full planning toolchain.
fn planning_scenario() -> String {
    use zbobr_dispatcher::mcp::planner_tools::{
        GET_DESCRIPTION, GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH,
        GET_PARAM_WORK_BRANCH, GET_PLAN, POST_PLAN, REPORT_RESULTS,
    };

    format!(
        r#"name: Planner Comprehensive Test
description: Verify all PLANNING MCP functions
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

- name: Get plan (initially empty)
  operation:
    type: tool_call
    tool: {GET_PLAN}
  assertions:
    - type: success

- name: Post implementation plan
  operation:
    type: tool_call
    tool: {POST_PLAN}
    arguments:
      description: "Step 1: analyse the codebase.\nStep 2: implement the feature.\nStep 3: write tests."
  assertions:
    - type: success

- name: Get plan (verify posted content)
  operation:
    type: tool_call
    tool: {GET_PLAN}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "analyse the codebase"

- name: Get destination branch (set via task update)
  operation:
    type: tool_call
    tool: {GET_PARAM_DESTINATION_BRANCH}
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: Get work branch (set via task update)
  operation:
    type: tool_call
    tool: {GET_PARAM_WORK_BRANCH}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "test"

- name: Report results and finish
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Planning complete. Implementation plan posted."
  assertions:
    - type: success
"#,
    )
}

async fn run_planning_test(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_planning").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Preparing)
        .await;

    let work_branch = format!("zbobr_fix-{task_id}-test");
    let repo_path_str = repo_path.to_string_lossy().to_string();
    env.update_task_branches(task_id, &repo_path_str, "main", &work_branch).await;
    env.prepare_workspace(task_id, &repo_path, &work_branch).await;

    // run the planning stage itself
    env.run_stage(task_id, Stage::Planning, planning_scenario()).await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Signal:      go_work"),
        "[{}] Planner follow-up signal should be GO_WORK after posting plan",
        env.backend_name()
    );

    // verify the work directory exists and is set up correctly
    let cloned_repo_path = env.workspaces_dir
        .join(format!("task#{task_id}"))
        .join("repo_planning");

    assert!(cloned_repo_path.exists(), "[{}] Work directory does not exist", env.backend_name());
    assert!(
        cloned_repo_path.starts_with(&env.workspaces_dir),
        "[{}] Work directory is not inside workspaces_dir",
        env.backend_name()
    );
    assert!(
        cloned_repo_path.join(".git").exists(),
        "[{}] Work directory is not a git repository",
        env.backend_name()
    );

    let branches_output = tokio::process::Command::new("git")
        .arg("branch")
        .current_dir(&cloned_repo_path)
        .output()
        .await
        .unwrap();
    let branches_str = String::from_utf8_lossy(&branches_output.stdout);

    assert!(
        branches_str.contains("main"),
        "[{}] Destination branch 'main' not found in cloned repo",
        env.backend_name()
    );

    let expected_work_branch = &work_branch;
    assert!(
        branches_str.contains(expected_work_branch.as_str()),
        "[{}] Work branch '{expected_work_branch}' not found in cloned repo",
        env.backend_name()
    );

    let current_branch_output = tokio::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&cloned_repo_path)
        .output()
        .await
        .unwrap();
    let current_branch = String::from_utf8_lossy(&current_branch_output.stdout)
        .trim()
        .to_string();
    assert_eq!(
        current_branch, *expected_work_branch,
        "[{}] Current branch is not the work branch",
        env.backend_name()
    );
}

#[tokio::test]
async fn test_planning() {
    let envs = IntegrationTestEnv::get_all().await;
    if envs.is_empty() {
        return;
    }
    for env in &envs {
        run_planning_test(env).await;
    }
}
