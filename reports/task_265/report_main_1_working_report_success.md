# Task Complete: push state on error

## Changes Made

### Fix 1 & 2: `zbobr-dispatcher/src/cli.rs` (commit `4e50a30`)
Both `process_task()` and `run_manager_loop()` now handle `runner.run()` errors gracefully:
1. Log error with `tracing::error!`
2. Format status with `format_error_status(zbobr, &msg)`
3. Call `task_session.set_pause_with_status_and_signal(status, Signal::go(stage_name))`
4. Log pause failure (if any) but don't propagate

This follows the same pattern as `finalize_stage_session()` which was the identified analog.

### Fix 3: Behavioral test `run_pause_on_runner_error` (commit `3a96c43`)
Added in `abstract_test_helpers.rs` and registered in `integration_fs_fs.rs`.

Uses empty description to trigger a pre-MCP pre-flight error. Two-step verification:
1. After `run_pipeline`: `pause=true`, `state=Running(Main, work)`, `signal=go("work")`, status contains "no description"
2. After `continue_pipeline`: `state=Pause`, `pause=false`, `signal=None`, `stack.len()==1`, `stack[0].signal=go("work")`

## Test Results
All workspace tests pass (0 failures).