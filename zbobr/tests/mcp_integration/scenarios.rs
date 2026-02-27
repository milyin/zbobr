/// Scenario YAML strings for every agent role.
/// Shared across all four backend-combination test files.

pub fn preparation_scenario(repo_path: &str) -> String {
    use zbobr_dispatcher::mcp::preparator_tools::{
        GET_DESCRIPTION, GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH,
        GET_PARAM_DESTINATION_REPOSITORY, GET_PARAM_WORK_BRANCH, SET_PARAM_DESTINATION_BRANCH,
        SET_PARAM_DESTINATION_REPOSITORY, SET_PARAM_WORK_BRANCH_POSTFIX,
    };

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

- name: Set work branch postfix
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
        repo_path = repo_path,
    )
}

pub fn planning_scenario() -> String {
    use zbobr_dispatcher::mcp::planner_tools::{
        GET_DESCRIPTION, GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH,
        GET_PLAN, POST_PLAN, REPORT_RESULTS,
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

- name: Get destination branch
  operation:
    type: tool_call
    tool: {GET_PARAM_DESTINATION_BRANCH}
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: Get work branch
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

pub fn working_scenario() -> String {
    use zbobr_dispatcher::mcp::worker_tools::{
        CHECK_CHECKLIST_ITEM, DELETE_CHECKLIST_ITEM, GET_CHECKLIST, GET_DESCRIPTION,
        GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH, GET_PLAN,
        INSERT_CHECKLIST_ITEM, REPORT_RESULTS, UPDATE_CHECKLIST_ITEM,
    };

    const PRIMARY_ID: &str = "w1";
    const TEMP_ID: &str = "w2";

    format!(
        r#"name: Worker Comprehensive Test
description: Verify core WORKING MCP functions
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

- name: Get plan
  operation:
    type: tool_call
    tool: {GET_PLAN}
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

- name: Get work branch
  operation:
    type: tool_call
    tool: {GET_PARAM_WORK_BRANCH}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "test"

- name: Get checklist (initially empty)
  operation:
    type: tool_call
    tool: {GET_CHECKLIST}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "[]"

- name: Insert primary checklist item
  operation:
    type: tool_call
    tool: {INSERT_CHECKLIST_ITEM}
    arguments:
      id: "{PRIMARY_ID}"
      text: "Implement worker stage integration coverage"
  assertions:
    - type: success

- name: Update primary checklist item
  operation:
    type: tool_call
    tool: {UPDATE_CHECKLIST_ITEM}
    arguments:
      id: "{PRIMARY_ID}"
      text: "Implement and validate worker stage integration coverage"
  assertions:
    - type: success

- name: Check primary checklist item
  operation:
    type: tool_call
    tool: {CHECK_CHECKLIST_ITEM}
    arguments:
      id: "{PRIMARY_ID}"
      checked: true
  assertions:
    - type: success

- name: Insert temporary checklist item
  operation:
    type: tool_call
    tool: {INSERT_CHECKLIST_ITEM}
    arguments:
      id: "{TEMP_ID}"
      text: "Temporary item to verify delete"
  assertions:
    - type: success

- name: Delete temporary checklist item
  operation:
    type: tool_call
    tool: {DELETE_CHECKLIST_ITEM}
    arguments:
      id: "{TEMP_ID}"
  assertions:
    - type: success
    - type: contains
      path: result
      value: "deleted"

- name: Report results and finish
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Worker complete."
  assertions:
    - type: success
"#,
    )
}

pub fn reviewing_scenario() -> String {
    use zbobr_dispatcher::mcp::reviewer_tools::{
        GET_DESCRIPTION, GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH, GET_PLAN,
        INSERT_CHECKLIST_ITEM, REPORT_RESULTS,
    };

    const GET_CHECKLIST: &str = "get_checklist";
    const REVIEW_ITEM_ID: &str = "r1";

    format!(
        r#"name: Reviewer Comprehensive Test
description: Verify core REVIEWING MCP functions
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

- name: Get plan
  operation:
    type: tool_call
    tool: {GET_PLAN}
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

- name: Get work branch
  operation:
    type: tool_call
    tool: {GET_PARAM_WORK_BRANCH}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "test"

- name: Get checklist (initially empty)
  operation:
    type: tool_call
    tool: {GET_CHECKLIST}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "[]"

- name: Insert review remark
  operation:
    type: tool_call
    tool: {INSERT_CHECKLIST_ITEM}
    arguments:
      id: "{REVIEW_ITEM_ID}"
      text: "Fix review issue: adjust edge-case handling"
  assertions:
    - type: success

- name: Report results and finish
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Reviewer complete."
  assertions:
    - type: success
"#,
    )
}

/// Scenario where the reviewer finds no issues (empty checklist → DONE + PR creation).
pub fn reviewing_approval_scenario() -> String {
    use zbobr_dispatcher::mcp::reviewer_tools::{GET_DESCRIPTION, GET_PLAN, REPORT_RESULTS};

    format!(
        r#"name: Reviewer Approval Test
description: Reviewer finds no issues — triggers DONE and PR creation
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

- name: Get plan
  operation:
    type: tool_call
    tool: {GET_PLAN}
  assertions:
    - type: success

- name: Report results and finish (no checklist items inserted)
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Reviewer approved. No issues found."
  assertions:
    - type: success
"#,
    )
}

pub fn merging_scenario(ending: &str) -> String {
    use zbobr_dispatcher::mcp::merger_tools::{
        ASK_USER, GET_DESCRIPTION, GET_DISCUSSION, GET_PARAM_DESTINATION_BRANCH,
        GET_PARAM_WORK_BRANCH, REPORT_RESULTS,
    };

    let ending_step = match ending {
        "report" => format!(
            r#"
- name: Report results and finish
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Merger complete."
  assertions:
    - type: success
"#
        ),
        "ask" => format!(
            r#"
- name: Ask user
  operation:
    type: tool_call
    tool: {ASK_USER}
    arguments:
      message: "Need guidance on merge"
  assertions:
    - type: success
"#
        ),
        _ => panic!("unknown merging ending: {ending}"),
    };

    format!(
        r#"name: Merger Comprehensive Test
description: Verify core MERGING MCP functions
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

- name: Get destination branch
  operation:
    type: tool_call
    tool: {GET_PARAM_DESTINATION_BRANCH}
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: Get work branch
  operation:
    type: tool_call
    tool: {GET_PARAM_WORK_BRANCH}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "test"
{ending_step}"#,
    )
}

pub fn merging_conflict_scenario() -> String {
    use zbobr_dispatcher::mcp::merger_tools::{GET_DESCRIPTION, REPORT_RESULTS};

    format!(
        r#"name: Merger Conflict Resolution Test
description: Test handling of real merge conflicts
timeout: 60
stop_on_failure: true

steps:
- name: Get task description
  operation:
    type: tool_call
    tool: {GET_DESCRIPTION}
  assertions:
    - type: success

- name: Report conflict resolution
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Detected merge conflicts in conflict_file.txt."
  assertions:
    - type: success
"#,
    )
}
