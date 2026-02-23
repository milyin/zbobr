use zbobr_dispatcher::mcp::preparator_tools::{
    GET_DESCRIPTION, GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH,
    GET_PARAM_DESTINATION_REPOSITORY, GET_PARAM_WORK_BRANCH, SET_PARAM_DESTINATION_BRANCH,
    SET_PARAM_DESTINATION_REPOSITORY, SET_PARAM_WORK_BRANCH_POSTFIX,
};

/// Inline scenario YAML for comprehensive preparator testing
pub fn preparator_comprehensive_scenario(repo_path: &str) -> String {
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
    )
}
