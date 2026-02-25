mod mcp_integration;

use mcp_integration::IntegrationTestEnv;
use zbobr_dispatcher::Stage;

/// Worker scenario that validates common worker tools and checklist workflow.
fn working_scenario() -> String {
    use zbobr_dispatcher::mcp::worker_tools::{
        CHECK_CHECKLIST_ITEM, DELETE_CHECKLIST_ITEM, GET_CHECKLIST, GET_DESCRIPTION,
        GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH, GET_PLAN,
        PUSH_WORK, REPORT_RESULTS, INSERT_CHECKLIST_ITEM, UPDATE_CHECKLIST_ITEM,
    };

    const CHECKLIST_ID_PRIMARY: &str = "w1";
    const CHECKLIST_ID_TEMP: &str = "w2";

    format!(
        r#"name: Worker Comprehensive Test
description: Verify core WORKING MCP functions
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

- name: Get plan
  operation:
    type: tool_call
    tool: {GET_PLAN}
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

- name: Get work branch
  operation:
    type: tool_call
    tool: {GET_PARAM_WORK_BRANCH}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "test"

- name: Get checklist initially
  operation:
    type: tool_call
    tool: {GET_CHECKLIST}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "[]"

- name: Insert primary checklist item
  operation:
    type: tool_call
    tool: {INSERT_CHECKLIST_ITEM}
    arguments:
      id: "{CHECKLIST_ID_PRIMARY}"
      text: "Implement worker stage integration coverage"
  assertions:
    - type: success

- name: Update primary checklist item
  operation:
    type: tool_call
    tool: {UPDATE_CHECKLIST_ITEM}
    arguments:
      id: "{CHECKLIST_ID_PRIMARY}"
      text: "Implement and validate worker stage integration coverage"
  assertions:
    - type: success

- name: Check primary checklist item
  operation:
    type: tool_call
    tool: {CHECK_CHECKLIST_ITEM}
    arguments:
      id: "{CHECKLIST_ID_PRIMARY}"
      checked: true
  assertions:
    - type: success

- name: Insert temporary checklist item
  operation:
    type: tool_call
    tool: {INSERT_CHECKLIST_ITEM}
    arguments:
      id: "{CHECKLIST_ID_TEMP}"
      text: "Temporary item to verify delete"
  assertions:
    - type: success

- name: Delete temporary checklist item
  operation:
    type: tool_call
    tool: {DELETE_CHECKLIST_ITEM}
    arguments:
      id: "{CHECKLIST_ID_TEMP}"
  assertions:
    - type: success
    - type: contains
      path: result
      value: "deleted"

- name: Push work branch
  operation:
    type: tool_call
    tool: {PUSH_WORK}
  assertions:
    - type: success

- name: Report results and finish
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Worker complete."
  assertions:
    - type: success
"#,
    )
}

#[tokio::test]
async fn test_working() {
    let Some(env) = IntegrationTestEnv::get().await else {
        return;
    };

    let repo_path = env.create_git_repo("repo_working").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Working)
        .await;

    let work_branch = format!("zbobr_fix-{task_id}-test");
    let repo_path_str = repo_path.to_string_lossy().to_string();
    env.update_task_branches(task_id, &repo_path_str, "main", &work_branch).await;
    env.prepare_workspace(task_id, &repo_path, &work_branch).await;

    env.run_stage(task_id, Stage::Working, working_scenario()).await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Worker complete."),
        "Worker report message was not recorded in discussion"
    );
    assert!(
        output.contains("Signal:      go_review"),
        "Worker follow-up signal should be GO_REVIEW when checklist has no unchecked items"
    );
    assert!(
        output.contains("[x] Implement and validate worker stage integration coverage"),
        "Expected checked checklist item was not found"
    );

    // verify the work directory exists and is set up correctly
    let cloned_repo_path = env.workspaces_dir
        .join(format!("task#{task_id}"))
        .join("repo_working");

    assert!(cloned_repo_path.exists(), "Work directory does not exist");
    assert!(
        cloned_repo_path.starts_with(&env.workspaces_dir),
        "Work directory is not inside workspaces_dir"
    );
    assert!(
        cloned_repo_path.join(".git").exists(),
        "Work directory is not a git repository"
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
        "Destination branch 'main' not found in cloned repo"
    );
    assert!(
        branches_str.contains(&work_branch),
        "Work branch '{work_branch}' not found in cloned repo"
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
        current_branch, work_branch,
        "Current branch is not the work branch"
    );
}
