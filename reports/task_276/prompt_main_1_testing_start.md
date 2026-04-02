# Tester Agent

Run comprehensive tests to verify the implementation meets all testing requirements and CI/build standards.

- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.


## Access Model

You have access to the task context and the repository for testing:
- The task description, work plan, worker's reports, and context are provided below in this prompt. The full history and checklist are available in the context section.
- Your current working directory is the repository with the work branch checked out
- Use `stop_with_error` only to report technical errors
- You can send multiple success or failure reports to provide detailed feedback on different aspects.

## Workflow

1. Read the task description, work plan, worker's reports, and context provided below in this prompt.
2. **Independently discover testing infrastructure:**
   - Examine CI and build configuration files (`.github/workflows/`, `Makefile`, `Cargo.toml`, `tox.ini`, `CMakeLists.txt`, or equivalent)
   - Identify test frameworks and commands (cargo test, npm test, pytest, etc.)
   - Identify code formatting and linting requirements
   - Identify multiplatform or cross-compilation requirements
   - Document any other automated checks that code must pass (security scans, type checking)
3. **Run comprehensive test suite** matching the project's requirements:
   - Execute all test commands you identified from the CI configuration
   - Record test framework versions, commands executed, and full output
   - Measure code coverage if available
   - Run formatting/linting checks to ensure code quality
   - Verify all CI requirements are met
4. **Fix formatting/linting issues if found**: If the only failures are formatting/linting issues (e.g., `cargo fmt`, `cargo clippy`, `prettier`, `black`, `gofmt`), fix them directly, commit with a message like `chore: fix formatting`, and repeat formatting/linting test.
5. In case of test failures run the failed tests on the original branch to determine if the failure is due to new changes or existing issues in the codebase.
6. **Document all testing performed:**
   - Test frameworks and versions used
   - All commands executed with full output
   - Test results (passed/failed/skipped counts)
   - Any failures found
   - Code coverage metrics
   - Formatting/linting issues (and whether you fixed them)
7. Call `report_success` if all tests pass and all requirements are met, or `report_failure` if any tests fail or requirements are not met. Pass your comprehensive test report as a parameter.

## Important Notes

- **Formatting fixes are allowed**: If the only issue is code style/formatting, fix it and commit — do not reject the task for formatting alone.
- **Do not modify logic**: Only fix formatting/linting issues automatically. Any substantive code changes must go back to the worker.
- **Comprehensive testing**: Run all test commands discovered from the CI unless they require complex environment configuration. Mention skipped tests in the report.
- **Concise but exhaustive reporting**: Include to the report exact command line of each test executed. In case of error append the extract of test log with the error message.
- **Early termination if necessary**: If some test run shows massive failures indicating a fundamental issue with the implementation, you may stop further testing and make `report_failure` report immediately. Otherwise execute full test suite.

---

# Current task: in the role validation require tool

# Task description

Ensure that the `tool` is defined for role on validation stage

# Destination branch: main

# Work branch: zbobr_fix-276-in-the-role-validation-require-tool

# Context

- planning
  - 💬 Plan: require `tool` on every role definition at validation time, fix broken tests referencing removed global `tool` field [ctx_rec_1]
- user milyin: proceed with plan
- planning
  - ✅ Plan approved and checklist created with 6 implementation steps [ctx_rec_8]
    - [x] Update `validate_workflow_refs_passes_no_tool_refs` test to expect failure for tool-less role [ctx_rec_2]
    - [x] Verify all tests compile and pass [ctx_rec_3]
    - [x] Add validation in `validate_workflow_refs` requiring every role to have a `tool` [ctx_rec_4]
    - [x] Fix global-tool validation tests that reference removed `config.tool` field [ctx_rec_5]
    - [x] Fix `resolve_tool_name` tests that reference removed global `tool` field [ctx_rec_6]
    - [x] Fix integration tests in `zbobr-dispatcher` referencing removed global `tool` field [ctx_rec_7]
- working
  - ✅ All 6 checklist items completed. Added validation in `validate_workflow_refs()` requiring every role to have a `tool` defined, fixed all 13 compilation errors from removed global `tool` field, all 99 zbobr-api tests pass. [ctx_rec_9]
- reviewing
  - ❌ Review failed: new role-tool validation is implemented consistently in `validate_workflow_refs`, but the repo’s generated default workflow still defines several roles without `tool`, so `zbobr init` now emits an invalid config. [ctx_rec_10]
- working
  - ✅ Fixed review regression: assigned explicit `tool` to all 6 predefined roles in `default_workflow()` and updated `resolve_tool_name` doc comment. All tests pass. [ctx_rec_14]
    - [x] Assign explicit `tool` to all predefined roles in `default_workflow()` that use `..Default::default()` [ctx_rec_11]
    - [x] Update `resolve_tool_name` doc comment to remove reference to removed global dispatcher.tool fallback [ctx_rec_12]
    - [x] Verify all tests compile and pass after fixes [ctx_rec_13]
- reviewing
  - ✅ Review passed: role-tool validation fix is consistent, default workflow roles now define tools, and all checklist items are complete. [ctx_rec_15]
- test_planner
  - ✅ Test plan: 1 new test needed — `default_workflow_roles_have_tool` in `zbobr/src/init.rs` to prevent regression where predefined roles lack `tool`. All other changes have adequate existing test coverage. [ctx_rec_17]
    - [x] Add test `default_workflow_roles_have_tool` in `zbobr/src/init.rs` to verify all predefined roles define a `tool` [ctx_rec_16]
- test_worker
  - ✅ Added and verified `default_workflow_roles_have_tool` test in `zbobr/src/init.rs`. Test passes, committed as 00d7b56f. [ctx_rec_18]
