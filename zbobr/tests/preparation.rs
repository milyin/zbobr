mod mcp_integration;

use mcp_integration::IntegrationTestEnv;
use zbobr_dispatcher::Stage;
use zbobr_dispatcher::mcp::preparator_tools::{
    GET_DESCRIPTION,
    GET_DISCUSSION,
    GET_PARAM_DESTINATION_REPOSITORY,
    GET_PARAM_DESTINATION_BRANCH,
    SET_PARAM_DESTINATION_REPOSITORY,
    SET_PARAM_DESTINATION_BRANCH,
    SET_PARAM_WORK_BRANCH_POSTFIX,
    GET_PARAM_WORK_BRANCH,
};

/// Inline scenario YAML for a comprehensive preparator test. Placing the
/// script directly in the test file ensures that the test is self-contained
/// and makes it easier to reason about what the preparator is doing.
fn preparation_scenario(repo_path: &str) -> String {
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
      value: "{repo_path}"
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
      value: "{repo_path}"

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
        repo_path = repo_path
    )
}

#[tokio::test]
async fn test_preparation() {
    let Some(env) = IntegrationTestEnv::get().await else {
        // mcp-tester not present; skip
        return;
    };

    let repo_path = env.create_git_repo("repo_preparation").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Preparing)
        .await;

    // run the preparator stage with the comprehensive scenario defined above
    env.run_stage(task_id, Stage::Preparing, preparation_scenario(&repo_path.to_string_lossy()))
        .await;

    let output = env.show_task(task_id).await;
    assert!(
      output.contains("Signal:      go_plan"),
      "Preparator follow-up signal should be GO_PLAN"
    );
}
