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
- name: get_history (no plan yet)
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success
    - type: contains
      path: result
      value: "Test task"

- name: post_plan
  operation:
    type: tool_call
    tool: post_plan
    arguments:
      description: "Plan: step A, step B"
  assertions:
    - type: success

- name: get_history (verify plan)
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success
    - type: contains
      path: result
      value: "step A"

- name: set_param_destination_repository
  operation:
    type: tool_call
    tool: set_param_destination_repository
    arguments:
      value: "{repo_path}"
  assertions:
    - type: success

- name: get_param_destination_repository
  operation:
    type: tool_call
    tool: get_param_destination_repository
  assertions:
    - type: success
    - type: equals
      path: result
      value: "{repo_path}"

- name: set_param_destination_branch
  operation:
    type: tool_call
    tool: set_param_destination_branch
    arguments:
      value: "main"
  assertions:
    - type: success

- name: get_param_destination_branch
  operation:
    type: tool_call
    tool: get_param_destination_branch
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: get_param_work_branch
  operation:
    type: tool_call
    tool: get_param_work_branch
  assertions:
    - type: success

- name: insert_checklist_item
  operation:
    type: tool_call
    tool: insert_checklist_item
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

- name: update_checklist_item
  operation:
    type: tool_call
    tool: update_checklist_item
    arguments:
      id: "c1"
      text: "Updated item"
  assertions:
    - type: success

- name: check_checklist_item
  operation:
    type: tool_call
    tool: check_checklist_item
    arguments:
      id: "c1"
      checked: true
  assertions:
    - type: success

- name: insert then delete
  operation:
    type: tool_call
    tool: insert_checklist_item
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

- name: report_results
  operation:
    type: tool_call
    tool: report_results
    arguments:
      message: "All tools exercised."
  assertions:
    - type: success
"#,
        repo_path = repo_path,
    )
}

/// Minimal scenario that just reports results (used for default-transition stages).
pub fn report_and_finish_scenario() -> String {
    r#"name: Report And Finish
description: Minimal scenario that reports results
timeout: 60
stop_on_failure: true

steps:
- name: Report results
  operation:
    type: tool_call
    tool: report_results
    arguments:
      message: "Stage complete."
  assertions:
    - type: success
"#
    .to_string()
}

/// Scenario that calls report_error (triggers PAUSE).
pub fn report_error_scenario() -> String {
    r#"name: Report Error
description: Report an error to trigger pause
timeout: 60
stop_on_failure: true

steps:
- name: Report error
  operation:
    type: tool_call
    tool: report_error
    arguments:
      message: "Something went wrong"
  assertions:
    - type: success
"#
    .to_string()
}

/// Scenario that calls review_reject (maps to a transition signal).
pub fn signal_reject_scenario() -> String {
    r#"name: Signal Reject
description: Call review_reject to trigger reject transition
timeout: 60
stop_on_failure: true

steps:
- name: Reject
  operation:
    type: tool_call
    tool: review_reject
    arguments:
      message: "Rejected."
  assertions:
    - type: success
"#
    .to_string()
}

/// Scenario that calls review_accept (maps to accept transition).
pub fn signal_accept_scenario() -> String {
    r#"name: Signal Accept
description: Call review_accept to trigger accept transition
timeout: 60
stop_on_failure: true

steps:
- name: Accept
  operation:
    type: tool_call
    tool: review_accept
    arguments:
      message: "Accepted."
  assertions:
    - type: success
"#
    .to_string()
}

/// Scenario that calls ask_user (triggers PAUSE).
pub fn ask_user_scenario() -> String {
    r#"name: Ask User
description: Call ask_user to trigger pause
timeout: 60
stop_on_failure: true

steps:
- name: Ask user
  operation:
    type: tool_call
    tool: ask_user
    arguments:
      message: "Need input"
  assertions:
    - type: success
"#
    .to_string()
}
