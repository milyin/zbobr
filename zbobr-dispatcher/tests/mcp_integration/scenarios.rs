//! Scenario YAML strings for every agent role.
//! Shared across all backend-combination test files.

pub fn preparation_scenario(repo_path: &str) -> String {
    format!(
        r#"name: Preparator Comprehensive Test
description: Verify all PREPARATION MCP functions
timeout: 60
stop_on_failure: true

steps:
- name: Get plan (returns task description when no plan exists)
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success
    - type: contains
      path: result
      value: "Dummy task description"

- name: Set destination repository
  operation:
    type: tool_call
    tool: set_param_destination_repository
    arguments:
      value: "{repo_path}"
  assertions:
    - type: success

- name: Get destination repository
  operation:
    type: tool_call
    tool: get_param_destination_repository
  assertions:
    - type: success
    - type: equals
      path: result
      value: "{repo_path}"

- name: Set destination branch
  operation:
    type: tool_call
    tool: set_param_destination_branch
    arguments:
      value: "main"
  assertions:
    - type: success

- name: Get destination branch
  operation:
    type: tool_call
    tool: get_param_destination_branch
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: Set work branch postfix
  operation:
    type: tool_call
    tool: set_param_work_branch_postfix
    arguments:
      value: "test"
  assertions:
    - type: success

- name: Get work branch
  operation:
    type: tool_call
    tool: get_param_work_branch
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
    format!(
        r#"name: Planner Comprehensive Test
description: Verify all PLANNING MCP functions
timeout: 60
stop_on_failure: true

steps:
- name: Get plan (initially returns task description)
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success

- name: Post implementation plan
  operation:
    type: tool_call
    tool: post_plan
    arguments:
      description: "Step 1: analyse the codebase.\nStep 2: implement the feature.\nStep 3: write tests."
  assertions:
    - type: success

- name: Get plan (verify posted content)
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success
    - type: contains
      path: result
      value: "analyse the codebase"

- name: Insert checklist item for worker
  operation:
    type: tool_call
    tool: insert_checklist_item
    arguments:
      id: "step-1"
      text: "Analyse the codebase"
  assertions:
    - type: success

- name: Get destination branch
  operation:
    type: tool_call
    tool: get_param_destination_branch
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: Get work branch
  operation:
    type: tool_call
    tool: get_param_work_branch
  assertions:
    - type: success
    - type: contains
      path: result
      value: "test"

- name: Post plan as final action (finishes session)
  operation:
    type: tool_call
    tool: post_plan
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
    format!(
        r#"name: Worker With Unchecked Item
description: Insert an unchecked item and finish
timeout: 60
stop_on_failure: true

steps:
- name: Insert unchecked checklist item
  operation:
    type: tool_call
    tool: insert_checklist_item
    arguments:
      id: "u1"
      text: "Unchecked work item"
  assertions:
    - type: success

- name: Report results without checking item
  operation:
    type: tool_call
    tool: report_results
    arguments:
      message: "Work reported with unchecked item."
  assertions:
    - type: success
"#,
    )
}

pub fn working_scenario() -> String {
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
    tool: get_history
  assertions:
    - type: success

- name: Get destination branch
  operation:
    type: tool_call
    tool: get_param_destination_branch
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: Get work branch
  operation:
    type: tool_call
    tool: get_param_work_branch
  assertions:
    - type: success
    - type: contains
      path: result
      value: "test"

- name: Get checklist (initially empty)
  operation:
    type: tool_call
    tool: get_checklist
  assertions:
    - type: success
    - type: contains
      path: result
      value: "[]"

- name: Insert primary checklist item
  operation:
    type: tool_call
    tool: insert_checklist_item
    arguments:
      id: "{PRIMARY_ID}"
      text: "Implement worker stage integration coverage"
  assertions:
    - type: success

- name: Update primary checklist item
  operation:
    type: tool_call
    tool: update_checklist_item
    arguments:
      id: "{PRIMARY_ID}"
      text: "Implement and validate worker stage integration coverage"
  assertions:
    - type: success

- name: Check primary checklist item
  operation:
    type: tool_call
    tool: check_checklist_item
    arguments:
      id: "{PRIMARY_ID}"
      checked: true
  assertions:
    - type: success

- name: Insert temporary checklist item
  operation:
    type: tool_call
    tool: insert_checklist_item
    arguments:
      id: "{TEMP_ID}"
      text: "Temporary item to verify delete"
  assertions:
    - type: success

- name: Delete temporary checklist item
  operation:
    type: tool_call
    tool: delete_checklist_item
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
    tool: report_results
    arguments:
      message: "Worker complete."
  assertions:
    - type: success
"#,
    )
}

pub fn reviewing_scenario() -> String {
    format!(
        r#"name: Reviewer Comprehensive Test
description: Verify core REVIEWING MCP functions
timeout: 60
stop_on_failure: true

steps:
- name: Get plan
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success

- name: Get destination branch
  operation:
    type: tool_call
    tool: get_param_destination_branch
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: Get work branch
  operation:
    type: tool_call
    tool: get_param_work_branch
  assertions:
    - type: success
    - type: contains
      path: result
      value: "test"

- name: Reject review (simulate discovered issue)
  operation:
    type: tool_call
    tool: review_reject
    arguments:
      message: "Reviewer complete. Found a problem during review."
  assertions:
    - type: success
"#,
    )
}

/// Scenario where the reviewer finds no issues — task should be marked DONE instead
/// of routing back to the planner.
pub fn reviewing_approval_scenario() -> String {
    format!(
        r#"name: Reviewer Approval Test
description: Reviewer finds no issues — task will be marked DONE
timeout: 60
stop_on_failure: true

steps:
- name: Get plan
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success

- name: Accept review (no issues found)
  operation:
    type: tool_call
    tool: review_accept
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
    format!(
        r#"name: Planner Report Error Test
description: Planner reports an error to verify retry signal
timeout: 60
stop_on_failure: true

steps:
- name: Report error
  operation:
    type: tool_call
    tool: report_error
    arguments:
      message: "Something went wrong during planning"
  assertions:
    - type: success
"#,
    )
}

pub fn merging_scenario(ending: &str) -> String {
    let ending_step = match ending {
        "report" => r#"
- name: Report results and finish
  operation:
    type: tool_call
    tool: report_results
    arguments:
      message: "Merger complete."
  assertions:
    - type: success
"#
        .to_string(),
        "ask" => r#"
- name: Ask user
  operation:
    type: tool_call
    tool: ask_user
    arguments:
      message: "Need guidance on merge"
  assertions:
    - type: success
"#
        .to_string(),
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
    tool: get_history
  assertions:
    - type: success

- name: Get destination branch
  operation:
    type: tool_call
    tool: get_param_destination_branch
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: Get work branch
  operation:
    type: tool_call
    tool: get_param_work_branch
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
    format!(
        r#"name: Worker Report Error Test
description: Verify report_error sets pause without clearing signal
timeout: 60
stop_on_failure: true

steps:
- name: Report error
  operation:
    type: tool_call
    tool: report_error
    arguments:
      message: "Something went wrong during work"
  assertions:
    - type: success
"#,
    )
}

/// Scenario for testing multiple plan postings and GET_HISTORY with offset parameter.
///
/// The `description` parameter is the task description, used to verify that
/// GET_HISTORY returns it as a user request comment when no plan has been posted.
pub fn multiple_plans_scenario(description: &str) -> String {
    format!(
        r#"name: Multiple Plans History Test
description: Verify GET_HISTORY with offset parameter and plan isolation
timeout: 60
stop_on_failure: true

steps:
- name: Get plan before any plan exists (returns task description as user request comment)
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success
    - type: contains
      path: result
      value: "{description}"

- name: Post first plan
  operation:
    type: tool_call
    tool: post_plan
    arguments:
      description: "First plan: step A, then step B"
  assertions:
    - type: success

- name: Add error comment between plans (simulates activity between plan versions)
  operation:
    type: tool_call
    tool: report_error
    arguments:
      message: "Issue found after first plan: needs revision"
  assertions:
    - type: success

- name: Post second plan
  operation:
    type: tool_call
    tool: post_plan
    arguments:
      description: "Second plan: revised step X, then step Y"
  assertions:
    - type: success

- name: Get latest history (default, no offset) - should return both plans (single chunk, no cuts)
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success
    - type: contains
      path: result
      value: "First plan"
    - type: contains
      path: result
      value: "Second plan"
    - type: contains
      path: result
      value: "current_chunk"
    - type: contains
      path: result
      value: "last_chunk"

- name: Get offset 0 - should return same chunk (oldest = latest when single chunk)
  operation:
    type: tool_call
    tool: get_history
    arguments:
      offset: 0
  assertions:
    - type: success
    - type: contains
      path: result
      value: "First plan"
    - type: contains
      path: result
      value: "Second plan"

- name: Get offset 1 - should return out-of-range (no cut boundaries, single chunk)
  operation:
    type: tool_call
    tool: get_history
    arguments:
      offset: 1
  assertions:
    - type: failure
    - type: contains
      path: result
      value: "out of range"
"#,
        description = description,
    )
}

pub fn merging_conflict_scenario() -> String {
    format!(
        r#"name: Merger Conflict Resolution Test
description: Test handling of real merge conflicts
timeout: 60
stop_on_failure: true

steps:
- name: Get plan
  operation:
    type: tool_call
    tool: get_history
  assertions:
    - type: success

- name: Report conflict resolution
  operation:
    type: tool_call
    tool: report_results
    arguments:
      message: "Detected merge conflicts in conflict_file.txt."
  assertions:
    - type: success
"#,
    )
}
