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

# Current task: add `--select` flag for `task process`

# Task description

Like in list, if `--select` flag is passed to `task process`, make it select the highest priority task and process it

# Destination branch: main

# Work branch: zbobr_fix-292-add-select-flag-for-task-process

# Context

- planning
  - 💬 Plan: Add `--select` bool flag to the `Process` variant in commands.rs. When set [ctx_rec_1]
- user milyin: proceed with plan
- planning
  - ✅ Plan approved and checklist items created for `--select` flag on `task process` [ctx_rec_4]
    - [x] Add `--select` flag to `Process` command variant [ctx_rec_2]
    - [x] Update `process` command handler to support `--select` [ctx_rec_3]
- working
  - ✅ Added --select flag to `task process` command [ctx_rec_5]
- reviewing
  - ❌ Found one review issue: `task process` enforces `task` vs `--select` exclusivity [ctx_rec_6]
- working
  - ✅ Fixed parser-level exclusivity for `task process --select` vs positional `task` [ctx_rec_8]
    - [x] Fix: enforce `task` vs `--select` exclusivity in Clap parser layer, not at runti [ctx_rec_7]
- reviewing
  - ❌ Review failed: `task process` still enforces `task` vs `--select` exclusivity at [ctx_rec_9]
- working
  - ✅ Fixed `task process` parser-level exclusivity: `#[arg(conflicts_with = "select") [ctx_rec_11]
    - [x] Fix: move `task` vs `--select` exclusivity from runtime handler to Clap parser v [ctx_rec_10]
- reviewing
  - ❌ Blocking review issue: `task process` still enforces `task` vs `--select` exclus [ctx_rec_12]
- working
  - ✅ Fixed Clap-level exclusivity for `task process --select` vs positional `task` [ctx_rec_14]
    - [x] Fix: enforce `task` vs `--select` exclusivity in Clap parser layer (blocking rev [ctx_rec_13]
- reviewing
  - ✅ Review passed: `task process --select` is correctly implemented, scoped, and con [ctx_rec_15]
- test_planner
  - ✅ Test plan ready: one checklist item for Clap parse-level unit tests covering `ta [ctx_rec_17]
    - [ ] Add Clap parse-level unit tests for `task process` mutual exclusivity and flag p [ctx_rec_16]
