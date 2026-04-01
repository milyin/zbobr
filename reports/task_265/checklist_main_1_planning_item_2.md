Add a new test function `run_pause_on_runner_error` to `zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs`.

**Purpose:** Verify that when `runner.run()` fails with an infrastructure-level error (before MCP starts), the task is gracefully paused rather than left in an inconsistent state.

**How to trigger the error:** Create a task with an empty description string (`""`). This hits the pre-flight check in `CliStageRunner::run()` (around line 450 in cli.rs):
```rust
if task_snap.description.is_empty() {
    anyhow::bail!("Task #{} has no description — nothing for the agent to do", ...);
}
```
This error fires before the MCP server starts, making it a reliable pre-MCP trigger.

**Test structure (model after `run_pause_on_error`, Test 6):**

1. Create a git repo and a task — pass `""` as the description to `env.create_task`
2. Set work branch via `env.update_task_branches`
3. Build a single-stage workflow: `build_workflow(vec![StageDef::new("work", "role_work", "main")])`
4. Use an empty scenarios map (stage never reaches MCP, so no scenario is needed)
5. Call `env.run_pipeline(task_id, &workflow, &scenarios)` — this calls `process_task` once
6. Call `env.get_task` and assert:
   - `task.pause == true`
   - `task.state == State::Pending(Pipeline::Main)` (OR `State::Running(Pipeline::Main, ...)` — check whichever the implementation produces; the key is it is NOT Done)
   - `task.signal == Some(Signal::go("work"))` — signal to re-run the failed stage
   - `task.status` contains the error message (non-empty, contains relevant text)
7. Call `env.continue_pipeline(task_id, &workflow, &scenarios)` — triggers `apply_pause_to_state`
8. Call `env.get_task` and assert:
   - `task.state == State::Pause`
   - `task.pause == false` (flag cleared)
   - `task.signal.is_none()` (signal cleared)
   - `task.stack.len() == 1`
   - `task.stack[0].pipeline == Pipeline::Main`
   - `task.stack[0].signal == Signal::go("work")`

**Register in integration_fs_fs.rs:**

Add a test function following the existing pattern in `zbobr-dispatcher/tests/integration_fs_fs.rs`:

```rust
#[tokio::test]
async fn test_fs_fs_abstract_pause_on_runner_error() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(env) = get_env().await else {
        return;
    };
    abstract_test_helpers::run_pause_on_runner_error(&env).await;
}
```
