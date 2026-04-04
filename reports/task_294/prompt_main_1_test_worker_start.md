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

# Current task: made separate working stage for fixing linter issues

# Task description

No need to go through full workflow if linter find a problem. Make `linter_worker` step and direct linter error to it. In case of success it goes back to linter.


# Destination branch: main

# Work branch: zbobr_fix-294-made-separate-working-stage-for-fixing-linter-issu

# Context

- planning
  - 💬 Plan: Add `linter_worker` stage between `linting` and `testing` to handle linter [ctx_rec_1]
- user milyin: proceed with plan
- planning
  - ✅ Plan approved and checklist items created for `linter_worker` stage implementati [ctx_rec_8]
    - [x] Update `linting` stage: change `on_failure` from `working` to `linter_worker` [ctx_rec_2]
    - [x] Add `linter_worker` stage to `main_stages` between `linting` and `testing` [ctx_rec_3]
    - [x] Add `linter_worker` role definition in `init.rs` [ctx_rec_4]
    - [x] Update `LINTER_PROMPT` to be check-only (remove auto-fix logic) [ctx_rec_5]
    - [x] Add `LINTER_WORKER_PROMPT` constant and register it in `PROMPT_FILES` [ctx_rec_6]
    - [x] Build and verify compilation succeeds [ctx_rec_7]
- working
  - ✅ Implemented linter_worker stage. Build passes. [ctx_rec_9]
- reviewing
  - ❌ Review failed: new linter_worker prompt violates repo prompt/commit rules despit [ctx_rec_10]
- working
  - ✅ Fixed LINTER_WORKER_PROMPT: removed commit instruction and pipeline workflow ref [ctx_rec_11]
- reviewing
  - ❌ Review failed: `linter_worker` prompt in `zbobr/src/init.rs` still violates repo [ctx_rec_12]
- working
  - ✅ Fixed LINTER_WORKER_PROMPT: removed commit instruction and all pipeline stage na [ctx_rec_13]
- reviewing
  - ❌ Review failed: linting success now advances to linter_worker, creating a lint lo [ctx_rec_14]
- working
  - ✅ Fixed lint loop: added explicit linting.on_success = testing [ctx_rec_15]
- reviewing
  - ✅ Review passed: linter_worker stage is wired correctly and matches the planned de [ctx_rec_16]
- test_planner
  - ✅ Test plan ready: 3 checklist items covering default workflow validation, linting [ctx_rec_20]
    - [ ] Unit test: default_workflow() passes validate() [ctx_rec_17]
    - [ ] Unit tests: linting and linter_worker stage transition routing [ctx_rec_18]
    - [ ] Unit test: every default workflow role with a prompt is registered in PROMPT_FIL [ctx_rec_19]
