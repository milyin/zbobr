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

# Current task: init: add --force flag

# Task description

Add flag `--force` to `init` command. With this parameter always overwrite destination files instead of creating `.new` nearby

# Destination branch: main

# Work branch: zbobr_fix-307-init-add-force-flag

# Context

- planning
  - 💬 Plan: Add `--force` flag to `init` command, following the existing `Setup` comma [ctx_rec_1]
- user milyin: proceed with plan
- planning
  - ✅ Plan approved and checklist created for adding `--force` flag to `init` command [ctx_rec_6]
    - [x] Add `force` field to `Init` variant in `commands.rs` [ctx_rec_2]
    - [x] Pass `force` flag through `main.rs` to `init_workspace()` [ctx_rec_3]
    - [x] Update `init.rs`: accept `force` param, change `write_or_new` behavior [ctx_rec_4]
    - [x] Build and test [ctx_rec_5]
- working
  - ✅ Added `--force` (`-f`) flag to `init` command. When set, existing files with dif [ctx_rec_7]
- reviewing
  - ❌ Review found one issue: `init_workspace` rustdoc still documents old `.new`-only [ctx_rec_8]
- working
  - ✅ Fixed stale `init_workspace` rustdoc to document `force` flag behavior. [ctx_rec_9]
- reviewing
  - ✅ Review passed: `init --force` is implemented correctly, follows the `Setup` anal [ctx_rec_10]
- test_planner
  - ✅ Test plan created: 4 tests for `write_or_new` covering all branches (force overw [ctx_rec_15]
    - [x] Test `write_or_new` with `force=true`: overwrites existing file when content dif [ctx_rec_11]
    - [x] Test `write_or_new` with `force=false`: creates `.new` file when content differs [ctx_rec_12]
    - [x] Test `write_or_new` skips write when content is identical (regardless of force f [ctx_rec_13]
    - [x] Test `write_or_new` creates new file when path doesn't exist [ctx_rec_14]
- test_worker
  - ✅ All 4 write_or_new tests implemented and passing [ctx_rec_16]
- linting
  - ❌ Formatting/linting failed: cargo fmt produced diffs; clippy ran with warnings. [ctx_rec_17]
- working
  - ✅ Fixed formatting issues: applied `cargo fmt` to `init.rs` and `main.rs`. Our cra [ctx_rec_18]
- reviewing
  - ❌ Review found 1 issue: `tempfile` was added under `[dependencies]` in `zbobr/Carg [ctx_rec_19]
- working
  - ✅ Moved `tempfile` from `[dependencies]` to `[dev-dependencies]` in `zbobr/Cargo.t [ctx_rec_20]
- reviewing
  - ✅ Review passed: `init --force` is implemented correctly, matches the `Setup` anal [ctx_rec_21]
