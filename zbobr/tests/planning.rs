mod mcp_integration;

use mcp_integration::IntegrationTestEnv;
use zbobr_dispatcher::Stage;
use zbobr_dispatcher::mcp::preparator_tools::{
    SET_PARAM_DESTINATION_REPOSITORY,
    SET_PARAM_DESTINATION_BRANCH,
    SET_PARAM_WORK_BRANCH_POSTFIX,
};

/// Minimal preparator scenario used only to supply the planner with the
/// configuration it requires.  The full comprehensive preparator script is
/// unnecessary here; we only set the destination repository, destination
/// branch and work-branch postfix.
fn minimal_preparation_scenario(repo_path: &str) -> String {
    format!(
        r#"name: Minimal Preparator for Planning
description: Only set the params needed by the planning test
timeout: 30
stop_on_failure: true

steps:
- name: Set destination repository
  operation:
    type: tool_call
    tool: {SET_PARAM_DESTINATION_REPOSITORY}
    arguments:
      value: "{repo_path}"
  assertions:
    - type: success

- name: Set destination branch
  operation:
    type: tool_call
    tool: {SET_PARAM_DESTINATION_BRANCH}
    arguments:
      value: "main"
  assertions:
    - type: success

- name: Set work branch
  operation:
    type: tool_call
    tool: {SET_PARAM_WORK_BRANCH_POSTFIX}
    arguments:
      value: "test"
  assertions:
    - type: success
"#,
        repo_path = repo_path
    )
}

/// Planner scenario similar to the previous comprehensive script.  The test
/// depends on the preparator having set the expected parameters; this script
/// exercises the full planning toolchain including a `pull_work` call.
fn planning_scenario() -> String {
    // For brevity we re-use the same YAML that was previously defined in the
    // shared helper; duplicating it here keeps the tests independent.
    use zbobr_dispatcher::mcp::planner_tools::{
        GET_DESCRIPTION, GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH,
        GET_PARAM_WORK_BRANCH, GET_PLAN, POST_PLAN, PULL_WORK, REPORT_RESULTS,
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

- name: Get destination branch (set by preparator)
  operation:
    type: tool_call
    tool: {GET_PARAM_DESTINATION_BRANCH}
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: Get work branch (set by preparator)
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

- name: Report results and finish
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Planning complete. Implementation plan posted. PULL_WORK_RETURN_VALUE=${{pull_work_result}}"
  assertions:
    - type: success
"#,
    )
}

#[tokio::test]
async fn test_planning() {
    let Some(env) = IntegrationTestEnv::get().await else {
        return;
    };

    let repo_path = env.create_git_repo("repo_planning").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Preparation)
        .await;

    // run a minimal preparator to populate the task parameters used by the
    // planner script
    env.run_preparation(minimal_preparation_scenario(&repo_path.to_string_lossy()), task_id)
        .await;

    // run the planning stage itself
    env.run_planning(planning_scenario(), task_id).await;

    // now verify the clone and branches as before; this logic used to live in
    // the shared environment but is specific to the planning test so we bring
    // it up here.
    let output = env.show_task(task_id).await;

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

    let expected_work_branch = format!("zbobr_fix-{task_id}-test");
    assert!(
        branches_str.contains(&expected_work_branch),
        "Work branch '{expected_work_branch}' not found in cloned repo"
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
        current_branch, expected_work_branch,
        "Current branch is not the work branch"
    );
}
