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

# Current task: in the role validation require tool

# Task description

Ensure that the `tool` is defined for role on validation stage

# Destination branch: main

# Work branch: zbobr_fix-276-in-the-role-validation-require-tool

# Context

- planning
  - 💬 Plan: require `tool` on every role definition at validation time, fix broken tests referencing removed global `tool` field [ctx_rec_1]
- user milyin: proceed with plan
- planning
  - ✅ Plan approved and checklist created with 6 implementation steps [ctx_rec_8]
    - [x] Update `validate_workflow_refs_passes_no_tool_refs` test to expect failure for tool-less role [ctx_rec_2]
    - [x] Verify all tests compile and pass [ctx_rec_3]
    - [x] Add validation in `validate_workflow_refs` requiring every role to have a `tool` [ctx_rec_4]
    - [x] Fix global-tool validation tests that reference removed `config.tool` field [ctx_rec_5]
    - [x] Fix `resolve_tool_name` tests that reference removed global `tool` field [ctx_rec_6]
    - [x] Fix integration tests in `zbobr-dispatcher` referencing removed global `tool` field [ctx_rec_7]
- working
  - ✅ All 6 checklist items completed. Added validation in `validate_workflow_refs()` requiring every role to have a `tool` defined, fixed all 13 compilation errors from removed global `tool` field, all 99 zbobr-api tests pass. [ctx_rec_9]
- reviewing
  - ❌ Review failed: new role-tool validation is implemented consistently in `validate_workflow_refs`, but the repo’s generated default workflow still defines several roles without `tool`, so `zbobr init` now emits an invalid config. [ctx_rec_10]
- working
  - ✅ Fixed review regression: assigned explicit `tool` to all 6 predefined roles in `default_workflow()` and updated `resolve_tool_name` doc comment. All tests pass. [ctx_rec_14]
    - [x] Assign explicit `tool` to all predefined roles in `default_workflow()` that use `..Default::default()` [ctx_rec_11]
    - [x] Update `resolve_tool_name` doc comment to remove reference to removed global dispatcher.tool fallback [ctx_rec_12]
    - [x] Verify all tests compile and pass after fixes [ctx_rec_13]
- reviewing
  - ✅ Review passed: role-tool validation fix is consistent, default workflow roles now define tools, and all checklist items are complete. [ctx_rec_15]
