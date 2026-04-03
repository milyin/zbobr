# Implementation Complete

All changes made in `zbobr/src/init.rs` (single commit: 98033f8f).

## What was done

1. **"drudge" tool** added to `default_config_toml()`: copilot with `gpt-5-mini` (primary) + claude with `claude-haiku-4.5` (priority 0 backup).

2. **"linting" stage** inserted in main pipeline before "testing": role `linter`, `on_failure` → `working`.

3. **"linter" role** added with same MCP tools as tester (`StopWithError`, `ReportSuccess`, `ReportFailure`, `StopWithQuestion`, `GetCtxRec`), prompt `linter.md`, tool `drudge`.

4. **`LINTER_PROMPT`** constant created: instructs agent to discover lint/fmt setup from CI, run checks, auto-fix issues, commit, and report. Uses `get_ctx_rec_guidance!()` macro.

5. **`TESTER_PROMPT`** updated: removed all formatting/linting steps (steps 3 bullet, step 4, step 6 bullet, important notes about formatting fixes), added note "Linting and formatting checks are handled by a separate stage — do not run them here."

6. **`PROMPT_FILES`** updated with `("linter", LINTER_PROMPT)` and test updated to include `LINTER_PROMPT` in the get_ctx_rec validation loop.

## Test results
11/11 tests passing.