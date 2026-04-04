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

# Current task: append instance name for work directories

# Task description

Ensure directory separation between multiple instances by adding `{instance}` name after each directory path. I.e. place task workspaces in `{workspace_dir}/{instance}`, place base repository in `{repo_dir}/{instance}`

# Destination branch: main

# Work branch: zbobr_fix-305-append-instance-name-for-work-directories

# Context

- planning
  - ✅ Comprehensive exploration of zbobr project directory and repository configuratio [ctx_rec_1]
  - 💬 Plan: Append instance name to workspaces and repos_dir paths at the wiring point [ctx_rec_2]
- user milyin: do it
- planning
  - ✅ Plan approved. Append instance name to workspaces and repos_dir at the wiring po [ctx_rec_6]
    - [x] Append instance to workspaces and repos_dir paths in commands.rs wiring point [ctx_rec_3]
    - [x] Update test environment to apply instance to workspaces and repos_dir paths [ctx_rec_4]
    - [x] Run cargo test and cargo clippy to verify no regressions [ctx_rec_5]
- working
  - ✅ Appended instance name to workspaces and repos_dir paths at wiring point and in [ctx_rec_7]
- reviewing
  - ✅ Review passed: instance-scoped workspaces and repos_dir are implemented correctl [ctx_rec_8]
- test_planner
  - ✅ No additional tests required: the 4-line wiring-point change (PathBuf::join) is [ctx_rec_9]
- test_worker
  - ✅ All tests pass: 14 integration tests, 126 API tests, 94 dispatcher tests validat [ctx_rec_10]
- linting
  - ❌ Formatting/lint checks failed: cargo fmt reported diffs; clippy reported warning [ctx_rec_11]
- working
  - ✅ Fixed formatting and lint issues: cargo fmt and cargo clippy now pass clean [ctx_rec_13]
    - [x] Fix formatting and lint issues reported by cargo fmt and cargo clippy [ctx_rec_12]
- reviewing
  - ❌ Review failed: feature implementation is correct, but the branch contains unrela [ctx_rec_14]
- working
  - ✅ Reverted unrelated formatting changes from 4 files, branch now contains only tas [ctx_rec_16]
    - [x] Revert unrelated formatting changes from zbobr-api/src/config.rs, zbobr-api/src/ [ctx_rec_15]
- reviewing
  - ✅ Review passed: branch now contains only the intended instance-scoped workspaces/ [ctx_rec_17]
