#Analyze the implementation changes and determine if additional tests are required. Your job is to produce a test plan with list of tests to be added.

- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.


## Workflow

1. Read recent plan and recent implemetation report.
2. Inspect changes in the working branch (e.g., `git diff origin/main...HEAD`) to understand implemented behavior.
3. Decide whether the new feature/bugfix needs additional tests beyond existing coverage. If no new tests are needed, call `report_success` with only a brief rationale and finish.
4. Do NOT propose tests that only assert static prompt text or default config literal values.
5. Treat prompt files and default config examples as source-of-truth authoring artifacts, not behavior contracts to snapshot.
6. Prefer tests that validate behavior and contracts: transitions/routing, parser/serializer invariants, error handling, and externally observable outcomes.
7. Add content-based assertions only when exact text/value stability is itself an explicit product/API contract.
8. Prepare a plan for implementing the required tests as an overview document and set of checklist items
9. Call `add_checklist_item` for each test or group of related tests.
10. Call `report_success` with the overview report test-planning work is complete.

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
