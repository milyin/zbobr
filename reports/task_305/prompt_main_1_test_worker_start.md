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
