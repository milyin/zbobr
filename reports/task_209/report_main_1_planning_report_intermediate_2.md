# Test Plan Assessment

## Changes in scope (commit f620f3b2)

### 1. validate() in zbobr-api/src/config.rs
Removed `!self.tools.is_empty() &&` guard so empty `[tools]` section always fails validation.

**Test coverage — already complete:**
- `validate_rejects_when_tools_empty` — updated in the fix commit itself (renamed from `validate_passes_when_tools_empty`, assertion flipped to expect error)
- `validate_rejects_unknown_global_tool` — non-empty tools but tool missing → error
- `validate_passes_when_global_tool_exists` — passing case

All three branches covered. No gap.

### 2. Retry loop in zbobr-dispatcher/src/cli.rs
`CliStageRunner::run()` now wraps provider selection + execution in a loop: on `connectivity_failure`, excludes the provider and immediately retries with the next available one.

**Unit tests not feasible:**
- `CliStageRunner` is a private struct with no existing test infrastructure
- `run()` requires MCP servers, executors, worktrees, and full async setup
- The retry loop is purely coordination logic: `select_provider()` → run → if `connectivity_failure` → `exclude_provider()` + `continue`
- `select_provider` and exclusion behavior are already covered by 7 existing unit tests in lib.rs (all-excluded error, priority fallback, round-robin, exclusion skip)

Adding heavyweight mock infrastructure solely to test the `continue` branch would be disproportionate.

## Conclusion
No additional tests needed. All 253 existing tests should pass.