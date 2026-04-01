# Plan: Push State on Error During Stage Execution

## Problem

When `CliStageRunner::run()` fails (e.g., template parsing error, empty description pre-flight), the task is left stranded in `State::Running` instead of being gracefully paused. Two call sites in `zbobr-dispatcher/src/cli.rs` mishandle this:

1. **`process_task()` ~line 893**: `runner.run().await?` propagates the error — task stays in `State::Running`, `process_task()` returns `Err`.
2. **Manager loop ~line 1120**: catches error but only calls `set_task_status_with_log()` — sets status text but does NOT set `pause=true` or signal, so `apply_pause_to_state()` is never triggered.

## Fix (both call sites)

Mirror the pattern in `finalize_stage_session()` lines ~1592–1609 (the analog for execution errors):

When `runner.run()` returns `Err(e)`:
1. `format_error_status(zbobr, &error_msg)` — formats with timestamp prefix
2. `task_session.set_pause_with_status_and_signal(status, Signal::go(stage_name))` — sets `pause=true`, `signal=go(stage_name)`, status
3. `task_session.set_state(State::pending(pipeline_name))` — moves out of `State::Running`

On next `process_task()` call, `apply_pause_to_state()` pushes stack entry and sets `State::Pause`.

Error propagation style differs by call site:
- Call site 1 (`process_task`): `set_state()` propagated with `?`, `set_pause_with_status_and_signal` failure logged only
- Call site 2 (manager loop): both failures logged only (fire-and-forget style, matches surrounding code)

## New Test

**File:** `abstract_test_helpers.rs` → new `run_stage_runner_error_pauses_task`, registered in `integration_fs_fs.rs`

**Closest analog:** `run_pause_state_conversion` (two-step test: trigger pause flag → continue to convert to PAUSE state)

**Error trigger:** Create task with **empty description** (`""`). Pre-flight check at ~line 450 bails before MCP server starts. No temp files needed.

**Steps:**
1. Create task with empty description, set work branch
2. `run_pipeline()` → runner fails → after fix: `pause=true`, `state=Pending(Main)`, `signal=go("work")`
3. Assert: `pause==true`, state, signal, status contains error text
4. `continue_pipeline()` → `apply_pause_to_state()` converts to `State::Pause`
5. Assert: `pause==false`, `state==Pause`, `signal==None`, `stack.len()==1`, `stack[0].signal==go("work")`

**Before fix:** `run_pipeline()` panics (it calls `process_task().unwrap_or_else(|e| panic!(...))`). Test correctly fails before fix.

## Files to Modify

- `zbobr-dispatcher/src/cli.rs` (~lines 893, 1120–1127)
- `zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs`
- `zbobr-dispatcher/tests/integration_fs_fs.rs`
