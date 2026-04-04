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
