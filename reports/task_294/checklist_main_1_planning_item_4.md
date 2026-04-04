In `zbobr/src/init.rs`:

1. Add a new `const LINTER_WORKER_PROMPT: &str` constant with a prompt for the `linter_worker` role.

The prompt should describe:
- **Role**: Fix the linting and formatting issues reported by the linter stage
- **Workflow**:
  1. Read the task context and the linter's failure report (which lists the issues found)
  2. Discover the same lint/fmt commands by examining CI and build config (same discovery as the linter)
  3. Run the linting/formatting tools to confirm which issues remain
  4. Apply fixes — both auto-fixes (via formatter tools) and manual fixes for linting warnings/errors that require code changes
  5. Commit the fixes with a message like `chore: fix linting issues`
  6. Call `{mcp_report_success}` if fixes were applied (linter stage will re-verify)
  7. Call `{mcp_report_failure}` if some issues cannot be fixed (with details), which escalates to the general worker

- **Important constraints**:
  - Only fix formatting and linting issues — do not modify logic, tests, or functionality
  - Do not run tests — functional testing is handled by a separate stage
  - Follow the same `get_ctx_rec_guidance!()` macro pattern used in other prompts

2. Register in `PROMPT_FILES` array: add `("linter_worker", LINTER_WORKER_PROMPT)` entry alongside the other prompt entries.

**Pattern to follow**: The `LINTER_PROMPT` (for the check-only part) and `TEST_WORKER_PROMPT` (for the worker pattern) are the closest analogs. Use `concat!()` with `get_ctx_rec_guidance!()` macro as other prompts do.