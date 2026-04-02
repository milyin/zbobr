# Test Plan — push state on error

## Summary

The existing `run_pause_on_runner_error` test provides adequate behavioral coverage of the error→pause→stack-push flow. One additional test registration is needed.

## Analysis

### Two modified call sites in `cli.rs`:
1. **`process_task()`** (line ~893) — exercised by test framework via `run_pipeline()`/`continue_pipeline()`
2. **`run_manager_loop()`** (line ~1140) — daemon loop, NOT exercisable by test framework

### Existing test `run_pause_on_runner_error`:
- ✅ Triggers pre-MCP error via empty description
- ✅ Verifies: pause flag set, state remains Running, signal set to `go("work")`, status contains error message
- ✅ Verifies: state converts to Pause, stack entry created with resume signal
- ✅ Registered in `integration_fs_fs.rs`
- ❌ NOT registered in `integration_github_github.rs`

### Call Site 2 coverage:
The manager loop cannot be tested with the current framework (helpers only call `process_task()`). The error handling logic is identical between both sites — only control flow differs (break vs. continue). No new test infrastructure is proposed for this gap since:
- The logic is a verbatim copy
- Testing would require a daemon-level integration harness that doesn't exist
- The risk is low

## Checklist

1. **Register `run_pause_on_runner_error` in `integration_github_github.rs`** — the abstract helper already exists, just needs to be wired into the GitHub backend test suite following the same pattern as other tests.

## No additional fs-backend tests needed

The existing test thoroughly covers the behavioral contract:
- Error triggers graceful pause (not crash)
- State is preserved for resume
- Stack entry records the failed stage
- Resume signal is set correctly