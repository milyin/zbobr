# Test Coverage Analysis — push state on error

## What Was Changed
Two call sites in `cli.rs` now gracefully pause tasks when `runner.run()` fails:
- **Call Site 1**: `process_task()` (~line 893) — single-step task processor
- **Call Site 2**: `run_manager_loop()` (~line 1140) — CLI daemon loop

Both sites use identical error handling: `format_error_status()` + `set_pause_with_status_and_signal()`.

## Existing Test Coverage

`run_pause_on_runner_error` in `abstract_test_helpers.rs`:
- Uses empty description to trigger pre-MCP error
- Verifies pause flag, state, signal, and status message after error
- Verifies state conversion to `Pause` with stack entry on next step
- Registered in `integration_fs_fs.rs` ✅
- **NOT registered in `integration_github_github.rs`** ❌

## Coverage Gaps

### Gap 1: GitHub backend registration (minor)
The abstract test helper exists but is only wired into fs-fs tests. Should also be registered in `integration_github_github.rs`.

### Gap 2: Call Site 2 (`run_manager_loop`) — NOT testable
The test framework's `run_pipeline()`/`continue_pipeline()` helpers both call `process_task()` directly. There is no test infrastructure to exercise the manager loop. The error handling logic is identical between both call sites — only the control flow differs (break vs. continue). This is an architectural limitation, not a missing test.

## Conclusion
The existing `run_pause_on_runner_error` test provides good behavioral coverage of the error→pause→stack-push flow for the fs backend. One actionable item: register the test for the GitHub backend.