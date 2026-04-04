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

# Current task: suppres logs by default

# Task description

add flag --logs and show logs only if this parameter is passed. Do not output logs to stdout by default

# Destination branch: main

# Work branch: zbobr_fix-295-suppres-logs-by-default

# Context

- planning
  - 💬 Plan: Add `--logs` flag to `Cli` struct in `main.rs`, move tracing subscriber in [ctx_rec_1]
- user milyin: proceed with task
- planning
  - ✅ Plan approved and checklist created. Single file change in `zbobr/src/main.rs`: [ctx_rec_4]
    - [x] Add `--logs` flag to `Cli` struct and move tracing init after CLI parsing [ctx_rec_2]
    - [x] Verify: `--help` shows flag, default is silent, `--logs` enables output [ctx_rec_3]
- working
  - ✅ Added `--logs` flag to `zbobr` CLI. Logs are now suppressed by default; passing [ctx_rec_5]
- reviewing
  - ❌ Review failed: `--logs` was added in `zbobr/src/main.rs`, but it is not integrat [ctx_rec_6]
- working
  - ✅ Fixed `--logs` flag integration with global CLI hoisting. Added `logs` field to [ctx_rec_9]
    - [x] Add `logs` flag to `GlobalArgs` in `zbobr-dispatcher/src/cli.rs` so `--logs` is [ctx_rec_7]
    - [x] Verify both `zbobr --logs task process` and `zbobr task process --logs` parse co [ctx_rec_8]
- reviewing
  - ✅ Review passed: `--logs` is now correctly implemented as a global root flag and l [ctx_rec_10]
- test_planner
  - ✅ Test plan: 3 tests across 2 files — 2 Cli parsing tests in main.rs + 1 GlobalArg [ctx_rec_13]
    - [ ] Add `logs_flag_defaults_to_false` and `logs_flag_parses_when_present` tests in ` [ctx_rec_11]
    - [ ] Add `global_args_includes_logs_flag` test in `zbobr-dispatcher/src/cli.rs` to pr [ctx_rec_12]
