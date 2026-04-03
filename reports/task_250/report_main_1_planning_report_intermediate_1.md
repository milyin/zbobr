## Plan: Separate formatting/linting into its own stage

**Analog:** The existing "tester" stage/role pattern in `zbobr/src/init.rs`.

All changes are in **`zbobr/src/init.rs`**.

### 1. Add "drudge" tool definition

In `default_config_toml()`, add a new tool entry `"drudge"` to the `tools` IndexMap (alongside "developer", "planner", "helper", "reviewer"):
- copilot provider with model `gpt-5-mini`, default priority (primary)
- claude provider with model `claude-haiku-4.5`, priority 0 (backup)

This follows the same pattern as "helper" tool but with even cheaper models.

### 2. Add "linting" stage to main pipeline

In `default_workflow()`, insert a new stage `"linting"` in the `main_stages` IndexMap **before** `"testing"`:
- role: `"linter"`
- prompts: `task_prompt.clone()`
- on_failure: goes to `"working"` (same as testing)

The stage ordering becomes: planning → working → reviewing → test_planner → test_worker → **linting** → testing

### 3. Add "linter" role definition

Add a new role `"linter"` to the roles IndexMap:
- MCP tools: `StopWithError`, `ReportSuccess`, `ReportFailure`, `StopWithQuestion`, `GetCtxRec` (same as tester)
- prompt: `"linter.md"`
- tool: `"drudge"`

### 4. Create LINTER_PROMPT constant

New prompt focused on:
- Discovering formatting/linting setup by examining CI config files
- Running formatting and linting checks
- **Fixing** any formatting/linting issues found (commit with descriptive message)
- Reporting success/failure
- NOT modifying logic — only formatting/linting fixes

### 5. Update TESTER_PROMPT

Remove formatting/linting references from the tester prompt:
- Remove step 3 bullet about "Identify code formatting and linting requirements"
- Remove step 3 bullet about "Run formatting/linting checks to ensure code quality"
- Remove step 4 entirely (fix formatting/linting issues)
- Remove "Formatting/linting issues" from reporting section
- Remove "Formatting fixes are allowed" note
- Remove "Do not modify logic" note (specific to formatting)
- Add a note: "Linting and formatting checks are handled by a separate stage — do not run them here."

### 6. Register the new prompt file

Add `("linter", LINTER_PROMPT)` to the `PROMPT_FILES` array.

### 7. Update tests

- Add linter prompt to `default_prompts_reference_get_ctx_rec_when_role_has_access` test if it uses `get_ctx_rec`
- Existing tests should continue to pass; no structural test changes needed beyond verifying the new stage exists