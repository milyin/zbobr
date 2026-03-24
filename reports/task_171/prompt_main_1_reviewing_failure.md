# Reviewer Agent

Review the implementation changes and ensure they meet coding standards and task requirements.

## Access Model

    You have read-only access to the task plan and access to the repository for inspection:
    - The task description, work plan, worker's report, comments, and checklist are provided below in this prompt. Use `get_history` to read the full discussion history if needed for more context.
    - Your current working directory is already the repository with the work branch checked out — examine changes directly
    - Use `stop_with_error` only to report technical errors

## Workflow

1. Read the task description, work plan, worker's report, comments, and checklist provided below in this prompt. Note if the analog solution in the existing code is referenced in the plan.
2. **Inspect all changes made in this task**: Use `git diff origin/<destination_branch>...HEAD` (three dots) to see ALL changes introduced by this task relative to the base branch. Do NOT checkout the base branch (it may conflict with worktree setup). You can also use `git log origin/<destination_branch>..HEAD` to see all commits in this branch.
3. **Verify the analog choice and pattern consistency**: Check that the planner chose an appropriate analog for the new functionality. Then verify that the implementation consistently follows the same patterns, conventions, coding style, and architectural approaches as the analog. Flag any deviations — new code should look like it was written by the same author as the existing analogous code. If the analog was poorly chosen, note this as a review finding.
4. **Review code quality and correctness**: Examine the implementation for correctness, code style, design patterns, and adherence to the plan. **Do not run any tests yourself; testing is handled in a separate Testing stage.**
5. Verify that all changes are related to the task and are necessary for the implementation. Flag any extraneous changes that do not directly contribute to the task requirements or plan.
6. Prepare a detailed review report describing any issues found, suggested fixes, and overall assessment. Include your assessment of analog consistency.
7. Call `report_success` if the implementation is correct and complete, or `report_failure` if issues were found. Pass the review report as a parameter to these tools.

---

# Current task: `ERROR` section instead of posting error to comments

# Task description

The error reports should be placed to dedicated section ERROR, similar to PARAMETERS, instead of posting coment
New error should replace the previous one.
Filtering comments from errors when reading the history can be removed, as well as special error type of comment.

# Destination branch: main

# Work branch: zbobr_fix-171-error-section

# Last report

Implemented ERROR section in separator.rs (parse/serialize/merge), added `error: Option<String>` to Task struct, removed `HistoryRecordType::Error`, updated all call sites in github.rs and all Task construction sites across the codebase. All 4 separator tests pass, full `cargo check` passes. Items error-section-separator, task-struct-error, github-backend-update completed.

[report_main_1_working_success.md](https://github.com/milyin/zbobr/blob/reports/reports/task_171/report_main_1_working_success.md)

# Last request

The error reports should be placed to dedicated section ERROR, similar to PARAMETERS, instead of posting coment
New error should replace the previous one.
Filtering comments from errors when reading the history can be removed, as well as special error type of comment.

# Unchecked checklist items

- [ ] [id: backend-set-error] Add `set_error(error: Option<String>)` default method to `TaskMut` trait in `zbobr-api/src/backend.rs`, following the `set_pause` pattern.
- [ ] [id: fs-backend-update] Add `#[serde(default)] error: Option<String>` to `TaskFile` in `zbobr-task-backend-fs/src/fs.rs`. Include `error` in both `to_task` and `from_task` methods.
- [ ] [id: role-session-set-error] Add `set_error(error: Option<String>)` method to `RoleSession` in `zbobr-dispatcher/src/task.rs`, following the `set_pause` pattern.
- [ ] [id: rewrite-stop-with-error] Rewrite `stop_with_error_impl` in `zbobr-dispatcher/src/mcp/traits.rs`: replace `post_comment` call with `self.session().set_error(Some(message.to_string())).await`. Keep `set_pause(true)` call.
- [ ] [id: update-tests] Update test assertions in `zbobr-dispatcher/tests/mcp_integration/test_helpers.rs`: in `run_report_error_preserves_signal` (~line 626) and `run_exit_preserves_agent_set_signal` (~line 1647), assert `task.error` contains the error message instead of checking comments. Ensure `cargo test` passes.