use zbobr_dispatcher::mcp::planner_tools::{
    GET_DESCRIPTION, GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH, GET_PLAN,
    POST_PLAN, REPORT_RESULTS,
};

/// Inline scenario YAML for comprehensive planner testing.
///
/// Exercises all Planning-stage MCP tools except `pull_work` (git setup is
/// deferred to a future iteration).  Assumes the Preparation stage has already
/// run so that destination_branch is "main" and the work-branch postfix
/// contains "test".
pub fn planner_comprehensive_scenario() -> String {
    format!(
        r#"name: Planner Comprehensive Test
description: Verify all PLANNING MCP functions (except pull_work)
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

  - name: Report results and finish
    operation:
      type: tool_call
      tool: {REPORT_RESULTS}
      arguments:
        message: "Planning complete. Implementation plan posted."
    assertions:
      - type: success
"#,
    )
}
