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
