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

# Current task: Separate formatting/linting stage

# Task description

Move the formatting / linting check out of tester stage prompt. 
Create separate stage "linting" with prompt which asks to do linting and formatting. Put this stage before testing. This stage is allowed to fix linting/formatting issues. Ask it to look at CI to check the formatting/linting setup.
In the tester stage prompt explain that linting and formatting checks are not needed.

For the formatting/linting stage define tool "drudge" which use cheapest models - free gpt-5 mini for copilot and haiku for claude (with priority 0, as backup)

# Destination branch: main

# Work branch: zbobr_fix-250-separate-formatting-linting-stage

# Context

- planning
  - 💬 Plan: Add "linting" stage before "testing" with new "drudge" tool using cheap models. All changes in zbobr/src/init.rs. [ctx_rec_1]
- user milyin: proceed with plan
- planning
  - ✅ Plan approved. 6 checklist items created for implementation: add "drudge" tool, add "linting" stage, add "linter" role, create LINTER_PROMPT, update TESTER_PROMPT, update tests. [ctx_rec_8]
    - [x] Add "drudge" tool definition in default_config_toml() [ctx_rec_2]
    - [x] Add "linting" stage to main pipeline before "testing" [ctx_rec_3]
    - [x] Add "linter" role definition [ctx_rec_4]
    - [x] Create LINTER_PROMPT constant and register in PROMPT_FILES [ctx_rec_5]
    - [x] Update TESTER_PROMPT to exclude formatting/linting [ctx_rec_6]
    - [x] Update tests for new linter stage and prompt [ctx_rec_7]
- working
  - ✅ All 6 checklist items implemented. New linting stage with drudge tool added before testing. All 11 tests pass. [ctx_rec_9]
- reviewing
  - ❌ Found one task-relevant gap: the new linting workflow wiring is implemented, but the tests were not updated to verify the new stage/order/tool wiring. [ctx_rec_10]
- working
  - ✅ Added 4 tests covering linting stage wiring: stage existence/order, linter role, drudge tool, and prompt. All 15 tests pass. [ctx_rec_12]
    - [x] Add tests verifying linting stage wiring (stage exists, order, role, prompt, drudge tool) [ctx_rec_11]
- reviewing
  - ❌ Review failed: linting stage wiring is correct, but test coverage still misses the exact drudge model/priority contract required by the task. [ctx_rec_13]
- working
  - ✅ Strengthened drudge tool tests to verify exact model/priority contract. All 15 tests pass. [ctx_rec_15]
    - [x] Strengthen drudge tool tests to verify exact model/priority contract (copilot=gpt-5-mini, claude=claude-haiku-4.5 with priority=0) [ctx_rec_14]
- reviewing
  - ✅ Review passed: linting stage, drudge tool, linter role, prompt registration, tester prompt cleanup, and exact drudge model/priority tests all match the task. [ctx_rec_16]
