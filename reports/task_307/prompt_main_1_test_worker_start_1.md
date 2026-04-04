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
- test_planner
  - ✅ No additional tests needed. All 4 branches of `write_or_new` are already covered [ctx_rec_22]
