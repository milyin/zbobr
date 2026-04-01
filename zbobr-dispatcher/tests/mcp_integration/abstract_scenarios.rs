//! Abstract scenario YAML strings for pipeline tests.
//!
//! These scenarios use generic role/stage names (not "planning", "working", etc.)
//! to test the pipeline machinery independently of specific naming conventions.

/// Scenario that exercises all available MCP tools.
pub fn all_mcp_tools_scenario(_repo_path: &str) -> String {
    r#"name: All MCP Tools Test
description: Exercise every MCP tool in a single session
timeout: 60
stop_on_failure: true

steps:
- name: add_checklist_item
  operation:
    type: tool_call
    tool: add_checklist_item
    arguments:
      brief: "First item"
      full_report: "Detailed description of the first checklist item."
  assertions:
    - type: success
    - type: contains
      path: result
      value: "ctx_rec_"

- name: check_checklist_item
  operation:
    type: tool_call
    tool: check_checklist_item
    arguments:
      id: "1"
  assertions:
    - type: success
    - type: contains
      path: result
      value: "checked"

- name: get_ctx_rec
  operation:
    type: tool_call
    tool: get_ctx_rec
    arguments:
      id: "1"
  assertions:
    - type: success

- name: report_success
  operation:
    type: tool_call
    tool: report_success
    arguments:
      brief: "All tools exercised."
      full_report: "Detailed report: all MCP tools were tested successfully."
  assertions:
    - type: success
"#
    .to_string()
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
