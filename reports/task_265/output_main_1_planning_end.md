## Plan Summary

**Problem:** When `CliStageRunner::run()` fails (template parsing error, empty description, etc.), the task is left in `State::Running` instead of being gracefully paused with state pushed to the stack.

**Fix — 2 call sites in `cli.rs`:**

1. **`process_task()` ~line 893**: Replace `runner.run().await?` with error handling that calls `set_pause_with_status_and_signal(status, Signal::go(stage_name))` + `set_state(State::pending(pipeline_name))` — mirrors the analog in `finalize_stage_session()` lines ~1592–1609.

2. **Manager loop ~line 1120**: Replace `set_task_status_with_log` with the same pause+pending pattern (both failures logged, not propagated — matches manager loop's fire-and-forget style).

**New behavioral test:**
- Triggers error via empty task description (pre-flight check bails before MCP starts)
- Two-step: `run_pipeline()` → assert `pause=true` + `Pending(Main)` + `signal=go("work")` → `continue_pipeline()` → assert `State::Pause` + stack entry
- Pattern follows `run_pause_state_conversion`

**Files:** `cli.rs` (2 spots), `abstract_test_helpers.rs` (new fn), `integration_fs_fs.rs` (register test)