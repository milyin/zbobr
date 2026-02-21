use zbobr_dispatcher::mcp::preparator_tools::GET_DESCRIPTION;

/// A sentinel scenario that always fails.
///
/// Passed to every executor-stage slot that must *not* be executed in a given
/// test run.  If zbobr accidentally routes execution through the wrong stage,
/// mcp-tester will run this scenario, hit the impossible assertion, exit with a
/// non-zero status, and the test will fail — making the routing error visible.
pub fn assert_false_scenario() -> String {
    format!(
        r#"name: Assert False - must not run
description: Sentinel scenario – always fails on execution
timeout: 30
stop_on_failure: true

steps:
  - name: This stage must not execute
    operation:
      type: tool_call
      tool: {GET_DESCRIPTION}
    assertions:
      - type: equals
        path: result
        value: "ASSERT_FALSE: this scenario must never be reached"
"#,
    )
}
