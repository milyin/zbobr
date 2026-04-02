## Plan: Separate formatting/linting into its own stage

### Context
Currently the tester stage prompt includes formatting/linting discovery, checking, and fixing. The task asks to extract this into a dedicated "linting" stage that runs before testing.

### Changes (all in `zbobr/src/init.rs`)

**1. Add "linting" stage to main pipeline stages**
Insert a new stage entry between "test_worker" and "testing" in the `main_stages` IndexMap:
- Stage name: `"linting"`
- Role: `"linter"`
- `on_failure`: transition to `"working"` (same pattern as testing)
- Include `task_prompt` in prompts

**2. Add "linter" role definition**
Add a new role entry in the `roles` IndexMap, following the tester role as the closest analog:
- MCP tools: `StopWithError`, `ReportSuccess`, `ReportFailure`, `StopWithQuestion`, `GetCtxRec` (same as tester — it needs to report failure if linting can't be fixed)
- Prompt file: `"linter.md"`
- Tool: `"smart"` (same as tester — it needs to edit/fix files)

**3. Create `LINTER_PROMPT` constant**
Write a new prompt constant following the tester prompt structure. Key points:
- Instruct the agent to examine CI/build config to discover formatting and linting setup
- Run formatting/linting checks (e.g. `cargo fmt --check`, `cargo clippy`, `prettier`, `black`, `gofmt`)
- Fix any formatting/linting issues found, commit with appropriate message
- Report success when all formatting/linting passes, or failure if issues can't be auto-fixed
- Do not modify logic — only formatting/linting fixes

**4. Register linter prompt file**
Add `("linter", LINTER_PROMPT)` to the `PROMPT_FILES` array.

**5. Update `TESTER_PROMPT`**
- Remove references to formatting/linting discovery (line 712)
- Remove "Run formatting/linting checks" (line 719)
- Remove the "Fix formatting/linting issues" step (line 721)
- Remove "Formatting/linting issues" from documentation list (line 729)
- Remove "Formatting fixes are allowed" note (line 734)
- Add a note explaining that formatting/linting checks are handled by a separate stage and are not needed here
