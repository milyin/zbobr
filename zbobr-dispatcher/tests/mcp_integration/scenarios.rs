//! Scenario YAML strings for every agent role.
//! Shared across all backend-combination test files.

pub fn preparation_scenario(repo_path: &str) -> String {
    use zbobr_dispatcher::mcp::preparator_tools::{
        GET_PARAM_DESTINATION_BRANCH,
        GET_PARAM_DESTINATION_REPOSITORY, GET_PARAM_WORK_BRANCH, GET_PLAN, SET_PARAM_DESTINATION_BRANCH,
        SET_PARAM_DESTINATION_REPOSITORY, SET_PARAM_WORK_BRANCH_POSTFIX,
    };

    format!(
        r#"name: Preparator Comprehensive Test
description: Verify all PREPARATION MCP functions
timeout: 60
stop_on_failure: true

steps:
- name: Get plan (returns task description when no plan exists)
  operation:
    type: tool_call
    tool: {GET_PLAN}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "Dummy task description"

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
        GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH,
        GET_PLAN, INSERT_CHECKLIST_ITEM, POST_PLAN,
    };

    format!(
        r#"name: Planner Comprehensive Test
description: Verify all PLANNING MCP functions
timeout: 60
stop_on_failure: true

steps:
- name: Get plan (initially returns task description)
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

- name: Insert checklist item for worker
  operation:
    type: tool_call
    tool: {INSERT_CHECKLIST_ITEM}
    arguments:
      id: "step-1"
      text: "Analyse the codebase"
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

- name: Post plan as final action (finishes session)
  operation:
    type: tool_call
    tool: {POST_PLAN}
    arguments:
      description: "Final plan: analyse codebase, implement feature, write tests."
  assertions:
    - type: success
"#,
    )
}

/// Minimal working scenario that leaves one unchecked checklist item.
/// Used to verify that exit rule 2.3 sets GoWork (has_unchecked=true).
pub fn working_scenario_with_unchecked_item() -> String {
    use zbobr_dispatcher::mcp::worker_tools::{INSERT_CHECKLIST_ITEM, REPORT_RESULTS};

    format!(
        r#"name: Worker With Unchecked Item
description: Insert an unchecked item and finish
timeout: 60
stop_on_failure: true

steps:
- name: Insert unchecked checklist item
  operation:
    type: tool_call
    tool: {INSERT_CHECKLIST_ITEM}
    arguments:
      id: "u1"
      text: "Unchecked work item"
  assertions:
    - type: success

- name: Report results without checking item
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Work reported with unchecked item."
  assertions:
    - type: success
"#,
    )
}

pub fn working_scenario() -> String {
    use zbobr_dispatcher::mcp::worker_tools::{
        CHECK_CHECKLIST_ITEM, DELETE_CHECKLIST_ITEM, GET_CHECKLIST,
        GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH, GET_PLAN,
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
        GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH, GET_PLAN,
        REPORT_RESULTS,
    };

    format!(
        r#"name: Reviewer Comprehensive Test
description: Verify core REVIEWING MCP functions
timeout: 60
stop_on_failure: true

steps:
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

/// Scenario where the reviewer finds no issues — triggers routing to planner.
pub fn reviewing_approval_scenario() -> String {
    use zbobr_dispatcher::mcp::reviewer_tools::{GET_PLAN, REPORT_RESULTS};

    format!(
        r#"name: Reviewer Approval Test
description: Reviewer finds no issues — triggers routing to planner
timeout: 60
stop_on_failure: true

steps:
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

/// Planning scenario where the planner reports an error (calls report_error).
/// Used to verify that the retry signal (GoPlan) overrides the normal exit
/// signal (GoWork) when the agent sets a signal mid-session.
pub fn planning_report_error_scenario() -> String {
    use zbobr_dispatcher::mcp::planner_tools::REPORT_ERROR;

    format!(
        r#"name: Planner Report Error Test
description: Planner reports an error to verify retry signal
timeout: 60
stop_on_failure: true

steps:
- name: Report error
  operation:
    type: tool_call
    tool: {REPORT_ERROR}
    arguments:
      message: "Something went wrong during planning"
  assertions:
    - type: success
"#,
    )
}

pub fn merging_scenario(ending: &str) -> String {
    use zbobr_dispatcher::mcp::merger_tools::{
        ASK_USER, GET_PARAM_DESTINATION_BRANCH,
        GET_PARAM_WORK_BRANCH, GET_PLAN, REPORT_RESULTS,
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
{ending_step}"#,
    )
}

/// Scenario that calls `report_error` during a worker session.
/// Used to verify that report_error sets the pause flag but leaves the signal intact.
pub fn worker_report_error_scenario() -> String {
    use zbobr_dispatcher::mcp::worker_tools::REPORT_ERROR;

    format!(
        r#"name: Worker Report Error Test
description: Verify report_error sets pause without clearing signal
timeout: 60
stop_on_failure: true

steps:
- name: Report error
  operation:
    type: tool_call
    tool: {REPORT_ERROR}
    arguments:
      message: "Something went wrong during work"
  assertions:
    - type: success
"#,
    )
}

pub fn merging_conflict_scenario() -> String {
    use zbobr_dispatcher::mcp::merger_tools::{GET_PLAN, REPORT_RESULTS};

    format!(
        r#"name: Merger Conflict Resolution Test
description: Test handling of real merge conflicts
timeout: 60
stop_on_failure: true

steps:
- name: Get plan
  operation:
    type: tool_call
    tool: {GET_PLAN}
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
