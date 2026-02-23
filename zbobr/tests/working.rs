mod mcp_integration;

use mcp_integration::IntegrationTestEnv;
use zbobr_dispatcher::Stage;

/// Worker scenario that validates common worker tools and checklist workflow.
fn working_scenario() -> String {
    use zbobr_dispatcher::mcp::worker_tools::{
        CHECK_CHECKLIST_ITEM, DELETE_CHECKLIST_ITEM, GET_CHECKLIST, GET_DESCRIPTION,
        GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH, GET_PLAN,
        INSERT_CHECKLIST_ITEM, PULL_WORK, PUSH_WORK, REPORT_RESULTS, UPDATE_CHECKLIST_ITEM,
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

- name: Pull work
  operation:
    type: tool_call
    tool: {PULL_WORK}
  store_result: pull_work_result
  assertions:
    - type: success

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
      message: "Worker complete. PULL_WORK_RETURN_VALUE=${{pull_work_result}}"
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
    let task_id_str = task_id.to_string();
    let repo_path_str = repo_path.to_string_lossy().to_string();
    env.run_zbobr(
        "task",
        &[
            "update",
            &task_id_str,
            "--dest-repo",
            &repo_path_str,
            "--dest-branch",
            "main",
            "--work-branch",
            &work_branch,
        ],
    )
    .await;

    env.run_working(working_scenario(), task_id).await;

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

    let mut pull_work_return_value = None;
    for line in output.lines() {
        if let Some(idx) = line.find("PULL_WORK_RETURN_VALUE=") {
            let val = line[idx + "PULL_WORK_RETURN_VALUE=".len()..].trim();
            let val = val.trim_end_matches('\'');
            pull_work_return_value = Some(val.to_string());
            break;
        }
    }
    let pull_work_return_value =
        pull_work_return_value.expect("PULL_WORK_RETURN_VALUE not found in task output");

    let parsed: serde_json::Value = serde_json::from_str(&pull_work_return_value)
        .expect("Failed to parse PULL_WORK_RETURN_VALUE as JSON");
    let path_str = parsed
        .get("result")
        .and_then(|v| v.as_str())
        .expect("result field not found or not a string");

    let cloned_repo_path = std::path::PathBuf::from(path_str);
    assert!(cloned_repo_path.exists(), "Cloned repo path does not exist");
    assert!(
        cloned_repo_path.starts_with(&env.workspaces_dir),
        "Cloned repo path is not inside workspaces_dir"
    );
    assert!(
        cloned_repo_path.join(".git").exists(),
        "Cloned repo is not a git repository"
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
