use zbobr_dispatcher::mcp::planner_tools::{PULL_WORK, REPORT_RESULTS};

pub fn planner_pull_work_scenario() -> String {
    format!(
        r#"name: Planner Pull Work Test
description: Test pull_work tool
timeout: 60
stop_on_failure: true

steps:
  - name: Pull work
    operation:
      type: tool_call
      tool: {PULL_WORK}
    store_result: pull_work_result
    assertions:
      - type: success

  - name: Report results
    operation:
      type: tool_call
      tool: {REPORT_RESULTS}
      arguments:
        message: "PULL_WORK_RETURN_VALUE=${{pull_work_result}}"
    assertions:
      - type: success
"#,
    )
}
