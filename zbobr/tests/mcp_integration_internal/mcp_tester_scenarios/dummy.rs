use zbobr_dispatcher::mcp::preparator_tools::GET_DESCRIPTION;

/// Inline scenario YAML for simple testing (dummy get_description test)
#[allow(dead_code)]
pub fn dummy_scenario() -> String {
    format!(
        r#"name: Dummy MCP Test
description: Verify get_description returns the expected task description
timeout: 30
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
"#,
    )
}
