In `zbobr/src/init.rs`, create a new `LINTER_PROMPT` constant and add it to the `PROMPT_FILES` array.

**What — LINTER_PROMPT:** Create a prompt focused on:
- Discovering formatting/linting setup by examining CI config files (.github/workflows/, Makefile, Cargo.toml, etc.)
- Running formatting and linting checks only (not tests)
- **Fixing** any formatting/linting issues found (commit with descriptive message like "chore: fix formatting")
- Reporting success/failure with details of what was checked and fixed
- Explicitly scoped: do NOT modify logic, do NOT run tests — only formatting/linting

Follow the TESTER_PROMPT style and structure. Include the `get_ctx_rec_guidance!()` macro. Use the same MCP tool placeholders pattern ({mcp_stop_with_error}, {mcp_report_success}, {mcp_report_failure}).

**What — PROMPT_FILES:** Add `("linter", LINTER_PROMPT)` to the PROMPT_FILES array (around line 547).

**Why:** The linter agent needs clear instructions to focus exclusively on formatting/linting, discover the project's linting setup from CI, and fix issues autonomously.