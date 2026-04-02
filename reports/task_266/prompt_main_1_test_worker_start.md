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

# Current task: verify the prompts on start

# Task description

On verification stage check that all prompts are correctly parsed with dummy task. The goal is to avoid postponed runtime errors, all incorrectness should be found on start

# Destination branch: main

# Work branch: zbobr_fix-266-verify-the-prompts-on-start

# Context

- planning
  - 💬 Plan: Add `validate_all_prompts()` on `ConfiguredPromptBuilder` that renders every stage's prompt with a dummy task at startup, catching template parse errors and undefined variables early. Remove dead `validate_stage_prompts`. Call from both startup paths in `commands.rs`. [ctx_rec_1]
- user milyin: go with plan
- planning
  - ✅ Plan approved and checklist created. 4 implementation steps: add validate_all_prompts() method, remove dead validation code, call at both startup paths, add tests. [ctx_rec_6]
    - [x] Add `validate_all_prompts()` method on `ConfiguredPromptBuilder` [ctx_rec_2]
    - [x] Remove dead `validate_stage_prompts` and `file_exists` from prompts.rs and lib.rs export [ctx_rec_3]
    - [x] Call `validate_all_prompts()` at startup in both code paths in commands.rs [ctx_rec_4]
    - [x] Add tests for `validate_all_prompts` in prompts.rs test module [ctx_rec_5]
- working
  - ✅ All 4 checklist items completed. Added `validate_all_prompts()` on `ConfiguredPromptBuilder`, removed dead `validate_stage_prompts`/`file_exists`, called validation at both startup paths in commands.rs. Tests item skipped per workflow rules (no new tests in this stage). All existing tests pass. [ctx_rec_7]
- reviewing
  - ❌ Review found one blocking issue: the planned `validate_all_prompts()` tests were not added, so the branch does not fully meet the approved task scope. [ctx_rec_8]
- working
  - ✅ Added 4 unit tests for `validate_all_prompts()` covering: valid templates pass, undefined variable fails, missing file fails, call stages skipped. All 21 tests pass. [ctx_rec_10]
    - [x] Add unit tests for `validate_all_prompts()` in prompts.rs: valid pass, undefined variable fails, missing file fails, call stages skipped [ctx_rec_9]
- reviewing
  - ✅ Review passed: implementation matches the approved plan, follows the intended prompt-rendering analog, and all checklist items are completed. [ctx_rec_11]
- test_planner
  - ✅ Test plan complete: 2 additional tests needed for error aggregation across stages and multi-pipeline iteration coverage. [ctx_rec_14]
    - [ ] Add test: multiple errors are aggregated across stages (not first-failure-stops) [ctx_rec_12]
    - [ ] Add test: validation iterates across multiple pipelines and error identifies pipeline name [ctx_rec_13]
