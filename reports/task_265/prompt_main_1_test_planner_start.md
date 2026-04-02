#Analyze the implementation changes and determine if additional tests are required. Your job is to produce a test plan with list of tests to be added.

- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.


## Workflow

1. Read recent plan and recent implemetation report.
2. Inspect changes in the working branch (e.g., `git diff origin/main...HEAD`) to understand implemented behavior.
3. Decide whether the new feature/bugfix needs additional tests beyond existing coverage. If no new tests are needed, call `report_success` with only a brief rationale and finish.
4. Prepare a plan for implementing the required tests as an overview document and set of checklist items
5. Call `add_checklist_item` for each test or group of related tests.
6. Call `report_success` with the overview report test-planning work is complete.

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
