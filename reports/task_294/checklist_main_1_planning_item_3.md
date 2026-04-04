In `zbobr/src/init.rs`, update `LINTER_PROMPT` so that the linter stage only checks for issues and reports them — it must NOT attempt to fix anything.

**What to change**:
- Remove the "Fix auto-fixable issues only" step (currently step 4 in the workflow)
- Remove the instruction to commit auto-fix changes
- Remove the "Fix issues autonomously" note
- When issues are found, call `{mcp_report_failure}` with a detailed report of ALL issues found (so `linter_worker` knows what to fix)
- When no issues are found, call `{mcp_report_success}`

**New workflow should be**:
1. Read task description and context
2. Discover formatting/linting setup from CI and build config
3. Run all formatting and linting checks
4. Report success if all checks pass, or report failure with a detailed list of issues if any checks fail

**Why**: With the new `linter_worker` stage handling fixes, the linter's role is purely verification. Having the linter also fix things would bypass the dedicated fixer and muddy the separation of concerns. The linter reports issues; `linter_worker` fixes them; the linter verifies the fix.

**Important note**: Keep the instruction that linter should NOT modify logic, tests, or functionality.