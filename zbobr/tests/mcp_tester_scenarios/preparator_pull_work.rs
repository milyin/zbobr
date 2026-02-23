use zbobr_dispatcher::mcp::preparator_tools::{
    SET_PARAM_DESTINATION_BRANCH, SET_PARAM_DESTINATION_REPOSITORY, SET_PARAM_WORK_BRANCH_POSTFIX,
};

pub fn preparator_pull_work_scenario(repo_path: &str) -> String {
    format!(
        r#"name: Preparator Pull Work Test
description: Setup parameters for pull_work
timeout: 60
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
        value: "test-feature"
    assertions:
      - type: success
"#,
    )
}
