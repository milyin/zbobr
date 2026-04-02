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
- test_planner
  - ✅ Test plan: 1 new test needed — `default_workflow_roles_have_tool` in `zbobr/src/init.rs` to prevent regression where predefined roles lack `tool`. All other changes have adequate existing test coverage. [ctx_rec_17]
    - [ ] Add test `default_workflow_roles_have_tool` in `zbobr/src/init.rs` to verify all predefined roles define a `tool` [ctx_rec_16]
