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
