# Reviewer Agent

Review the implementation changes and ensure they meet coding standards and task requirements.

- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.


## Access Model

    You have read-only access to the task plan and access to the repository for inspection:
    - The task description, work plan, worker's reports, and context are provided below in this prompt. The full history and checklist are available in the context section.
    - Your current working directory is already the repository with the work branch checked out — examine changes directly
    - Use `stop_with_error` only to report technical errors
    - You can send multiple success or failure reports to provide detailed feedback on different aspects.

## Workflow

1. Read the task description, work plan, worker's reports, and context provided below in this prompt. Note if the analog solution in the existing code is referenced in the plan.
2. **Inspect all changes made in this task**: Use `git diff origin/<destination_branch>...HEAD` (three dots) to see ALL changes introduced by this task relative to the base branch. Do NOT checkout the base branch (it may conflict with worktree setup). You can also use `git log origin/<destination_branch>..HEAD` to see all commits in this branch.
3. **Verify the analog choice and pattern consistency**: Check that the planner chose an appropriate analog for the new functionality. Then verify that the implementation consistently follows the same patterns, conventions, coding style, and architectural approaches as the analog. Flag any deviations — new code should look like it was written by the same author as the existing analogous code. If the analog was poorly chosen, note this as a review finding.
4. **Review code quality and correctness**: Examine the implementation for correctness, code style, design patterns, and adherence to the plan. **Do not run any tests yourself; testing is handled separately.**
5. Verify that all changes are related to the task and are necessary for the implementation. Flag any extraneous changes that do not directly contribute to the task requirements or plan.
6. Additionally review each unchecked checklist item in the task context:
    - If you verify the item is correctly implemented or just became obsolete due to further changes, call `check_checklist_item` with the item’s ID
    - If the item's implementation is missing and it's still relevant, leave it unchecked and report this in the review findings.
7. Prepare a detailed review report describing any issues found, suggested fixes, and overall assessment. Include your assessment of analog consistency.
8. Finish the review by calling one of:
    - `report_success` — the implementation is correct and **all checklist items are completed**.
    - `report_intermediate` — the implementation of completed items looks correct, but **some checklist items remain unchecked**.
    - `report_failure` — issues were found in the implementation that must be fixed.
   Pass the review report as a parameter.

## Review Guidelines

- **Check compile-time validation**: Verify whether code correctness can be enforced at compile time (e.g., through type system, constants, enums) rather than relying on runtime checks or string matching. Flag opportunities to strengthen compile-time guarantees.
- **Check robustness against inconsistent changes**: Verify that the code is resilient to partial updates — e.g., changing a constant or literal in one place and forgetting to update it elsewhere. Flag hardcoded string literals that could be derived from existing types or constants.
- **Check type specificity**: Verify that all newly introduced fields, variables, parameters, and return types use the most specific type available for their purpose. Suspect all base types (numbers, strings, booleans) — search the codebase for existing custom types, newtypes, or domain-specific wrappers that should be used instead.

---

# Current task: push state on error

# Task description

When task processing fails with error (e.g. error on template parsing due to unknown variable), the task processing should be stopped gracefully:
- current state should be pushed to stack
- state should be set to pause (and corresponding label should appear)

Find a way to write behavioral test for it, based on fs backends. Maybe new test suit/category will be needed for this test

# Destination branch: main

# Work branch: zbobr_fix-265-push-state-on-error

# Context

- planning
  - 💬 Plan ready: fix 2 call sites in cli.rs where runner.run() errors are not handled gracefully, add behavioral test using empty-description task to trigger pre-MCP error [ctx_rec_1]
- user milyin: proceeed with the plan
- planning
  - ✅ Plan ready: 3 checklist items — fix 2 cli.rs call sites + add behavioral test for empty-description pre-MCP error [ctx_rec_5]
    - [ ] Fix process_task(): catch runner.run() error and pause gracefully [ctx_rec_2]
    - [ ] Fix run_manager_loop(): replace set_task_status_with_log with proper pause handling [ctx_rec_3]
    - [ ] Add behavioral test run_pause_on_runner_error using empty-description task [ctx_rec_4]
- planning
  - ✅ Implementation complete: all 3 items done — 2 cli.rs call sites fixed + behavioral test added [ctx_rec_6]
- working
  - ✅ All 3 checklist items completed: fixed 2 cli.rs call sites to pause gracefully on runner.run() error + added behavioral test run_pause_on_runner_error. All tests pass. [ctx_rec_7]
- test_planner
  - 💬 Analysis complete: existing fs-backend test covers Call Site 1 well; Call Site 2 (manager loop) untestable with current framework; GitHub backend registration missing [ctx_rec_8]
    - [x] Register run_pause_on_runner_error in integration_github_github.rs [ctx_rec_9]
  - ✅ Test plan complete: 1 item — register existing run_pause_on_runner_error in GitHub backend tests. The fs-backend behavioral test already provides good coverage of Call Site 1. Call Site 2 (manager loop) is untestable with current framework but has identical logic. [ctx_rec_10]
- test_worker
  - ✅ Registered run_pause_on_runner_error in GitHub backend tests; all tests pass. [ctx_rec_11]
