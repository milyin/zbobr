## Plan: Separate formatting/linting stage

**Approach:** All changes in `zbobr/src/init.rs`. Add a new "linting" stage before "testing" in the main pipeline, with a dedicated "linter" role using the cheapest available models via a new "drudge" tool.

**Key design decisions:**
- **"drudge" tool** uses gpt-5-mini (copilot, primary) and claude-haiku-4.5 (claude, priority 0 backup) — cheapest models for mechanical formatting tasks
- **Stage placement:** linting runs after test_worker and before testing, so formatting is clean before functional tests run
- **on_failure:** routes back to "working" stage (same as testing)
- **TESTER_PROMPT cleanup:** all formatting/linting references removed, replaced with a note that linting is handled separately

**Analog:** The "tester" role/stage pattern — same MCP tools, same stage definition structure, similar prompt style.

**Checklist items:**
1. Add "drudge" tool definition in default_config_toml()
2. Add "linting" stage to main pipeline before "testing"
3. Add "linter" role definition
4. Create LINTER_PROMPT constant and register in PROMPT_FILES
5. Update TESTER_PROMPT to exclude formatting/linting
6. Update tests for new linter stage and prompt