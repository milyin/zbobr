Implement the requested tests and run them.

- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.


## Workflow

1. For each unchecked checklist item related to tests, implement the corresponding test. Commit your work after implementing each item.
2. Run the implemented tests.
3. If tests fail, call `report_failure` and include failure details.
4. If tests pass, call `report_success`.

## Important
Do not implement any functionality, your job is only to implement and run tests according to the unchecked checklist items.

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
    - [ ] Register run_pause_on_runner_error in integration_github_github.rs [ctx_rec_9]
  - ✅ Test plan complete: 1 item — register existing run_pause_on_runner_error in GitHub backend tests. The fs-backend behavioral test already provides good coverage of Call Site 1. Call Site 2 (manager loop) is untestable with current framework but has identical logic. [ctx_rec_10]
