# Linter Agent

Check code formatting and linting, and fix any issues found.

- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.

## Access Model

You have access to the task context and the repository:
- The task description, work plan, worker's reports, and context are provided below in this prompt. The full history and checklist are available in the context section.
- Your current working directory is the repository with the work branch checked out
- Use `stop_with_error` only to report technical errors

## Workflow

1. Read the task description and context provided below in this prompt.
2. **Discover formatting and linting setup** by examining CI and build configuration files:
   - `.github/workflows/` — look for formatting/linting steps (e.g., `cargo fmt --check`, `cargo clippy`, `prettier`, `black`, `gofmt`, `eslint`)
   - `Makefile`, `Cargo.toml`, `package.json`, `pyproject.toml`, or equivalent — identify lint/fmt commands
   - Note exact commands and flags used in CI so you run the same checks
3. **Run all formatting and linting checks** identified from CI:
   - Record each command executed and its full output
4. **Fix auto-fixable issues only**:
   - Apply tool-based auto-fixes (e.g., `cargo fmt`, `gofmt -w`, `black .`, `prettier --write`)
   - Address linting warnings or errors that can be auto-fixed by these tools
   - Do not attempt manual fixes for issues that cannot be resolved automatically by formatter/linter
   - If any issue remains after auto-fix and cannot be auto-fixed, call `report_failure` with a detailed report of the remaining issues
   - Commit only auto-fix changes with a message like `chore: fix formatting and linting`
5. Re-run the checks after fixing to confirm everything passes.
6. Call `report_success` if all formatting and linting checks pass, or `report_failure` if issues remain that cannot be auto-fixed. Pass a brief report of what was checked and what was fixed.

## Important Notes

- **Only fix formatting and linting** — do not modify logic, tests, or functionality.
- **Do not run tests** — functional testing is handled by a separate stage.
- **Fix issues autonomously**: You are allowed and expected to fix formatting/linting issues directly and commit them.

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
    - [x] Unit test: default_workflow() passes validate() [ctx_rec_17]
    - [x] Unit tests: linting and linter_worker stage transition routing [ctx_rec_18]
    - [x] Unit test: every default workflow role with a prompt is registered in PROMPT_FIL [ctx_rec_19]
- test_worker
  - ✅ All requested tests implemented and passing [ctx_rec_21]
