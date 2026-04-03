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
- test_planner
  - ✅ Two additional tests needed: verify TESTER_PROMPT no longer instructs formatting/linting (with "separate stage" note), and verify LINTER_PROMPT covers formatting/linting without testing overlap. [ctx_rec_19]
    - [ ] Add test: TESTER_PROMPT no longer instructs formatting/linting and states it's handled separately [ctx_rec_17]
    - [ ] Add test: LINTER_PROMPT covers formatting and linting as its core responsibility [ctx_rec_18]
