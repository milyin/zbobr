//! Abstract scenario YAML strings for pipeline tests.
//!
//! These scenarios use generic role/stage names (not "planning", "working", etc.)
//! to test the pipeline machinery independently of specific naming conventions.

/// Scenario that exercises all available MCP tools.
pub fn all_mcp_tools_scenario(repo_path: &str) -> String {
    format!(
        r#"name: All MCP Tools Test
description: Exercise every MCP tool in a single session
timeout: 60
stop_on_failure: true

steps:
- name: get_history
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success
    - type: contains
      path: result
      value: "task"

- name: configure_worktree
  operation:
    type: tool_call
    tool: configure_worktree
    arguments:
      destination_repository: "{repo_path}"
      destination_branch: "main"
  assertions:
    - type: success

- name: add_checklist_item
  operation:
    type: tool_call
    tool: add_checklist_item
    arguments:
      id: "c1"
      text: "First item"
  assertions:
    - type: success

- name: get_checklist
  operation:
    type: tool_call
    tool: get_checklist
  assertions:
    - type: success
    - type: contains
      path: result
      value: "First item"

- name: check_checklist_item
  operation:
    type: tool_call
    tool: check_checklist_item
    arguments:
      id: "c1"
  assertions:
    - type: success

- name: add then delete
  operation:
    type: tool_call
    tool: add_checklist_item
    arguments:
      id: "c2"
      text: "Temp item"
  assertions:
    - type: success

- name: delete_checklist_item
  operation:
    type: tool_call
    tool: delete_checklist_item
    arguments:
      id: "c2"
  assertions:
    - type: success
    - type: contains
      path: result
      value: "deleted"

- name: report_success
  operation:
    type: tool_call
    tool: report_success
    arguments:
      brief: "All tools exercised."
      full_report: "Detailed report: all MCP tools were tested successfully."
  assertions:
    - type: success
"#,
        repo_path = repo_path,
    )
}

/// Scenario that calls configure_worktree twice with identical values.
/// The second call must remain successful and report that values were already set.
pub fn configure_worktree_idempotent_scenario(repo_path: &str, postfix: &str) -> String {
    format!(
        r#"name: Configure Worktree Idempotent
description: Repeated configure_worktree calls with same parameters should be a no-op success
timeout: 60
stop_on_failure: true

steps:
- name: configure_worktree first call
  operation:
    type: tool_call
    tool: configure_worktree
    arguments:
      destination_repository: "{repo_path}"
      destination_branch: "main"
      work_branch_postfix: "{postfix}"
  assertions:
    - type: success

- name: configure_worktree repeated call
  operation:
    type: tool_call
    tool: configure_worktree
    arguments:
      destination_repository: "{repo_path}"
      destination_branch: "main"
      work_branch_postfix: "{postfix}"
  assertions:
    - type: success
    - type: contains
      path: result
      value: "values were already set"

- name: report_success
  operation:
    type: tool_call
    tool: report_success
    arguments:
      brief: "Idempotent configure_worktree verified."
      full_report: "Repeated configure_worktree calls with identical parameters remained successful."
  assertions:
    - type: success
"#,
        repo_path = repo_path,
        postfix = postfix,
    )
}

/// Minimal scenario that just reports success (used for default-transition stages).
pub fn report_and_finish_scenario() -> String {
    r#"name: Report And Finish
description: Minimal scenario that reports success
timeout: 60
stop_on_failure: true

steps:
- name: Report success
  operation:
    type: tool_call
    tool: report_success
    arguments:
      brief: "Stage complete."
      full_report: "Stage completed successfully."
  assertions:
    - type: success
"#
    .to_string()
}

/// Scenario that calls stop_with_error (triggers PAUSE).
pub fn stop_with_error_scenario() -> String {
    r#"name: Stop With Error
description: Report an error to trigger pause
timeout: 60
stop_on_failure: true

steps:
- name: Stop with error
  operation:
    type: tool_call
    tool: stop_with_error
    arguments:
      message: "Something went wrong"
  assertions:
    - type: success
"#
    .to_string()
}

/// Scenario that calls report_failure (maps to a transition signal).
pub fn report_failure_scenario() -> String {
    r#"name: Report Failure
description: Call report_failure to trigger failure transition
timeout: 60
stop_on_failure: true

steps:
- name: Failure
  operation:
    type: tool_call
    tool: report_failure
    arguments:
      brief: "Rejected."
      full_report: "Detailed rejection report."
  assertions:
    - type: success
"#
    .to_string()
}

/// Scenario that calls report_success (maps to success transition).
pub fn report_success_scenario() -> String {
    r#"name: Report Success
description: Call report_success to trigger success transition
timeout: 60
stop_on_failure: true

steps:
- name: Success
  operation:
    type: tool_call
    tool: report_success
    arguments:
      brief: "Accepted."
      full_report: "Detailed acceptance report."
  assertions:
    - type: success
"#
    .to_string()
}

/// Scenario that calls stop_with_question (triggers PAUSE).
pub fn stop_with_question_scenario() -> String {
    r#"name: Stop With Question
description: Call stop_with_question to trigger pause
timeout: 60
stop_on_failure: true

steps:
- name: Ask user
  operation:
    type: tool_call
    tool: stop_with_question
    arguments:
      message: "Need input"
  assertions:
    - type: success
"#
    .to_string()
}
