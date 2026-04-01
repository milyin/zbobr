All 3 checklist items were already fully implemented in the work branch and verified:

1. **`process_task()`** — `runner.run().await?` replaced with error-catching that calls `set_pause_with_status_and_signal`
2. **`run_manager_loop()`** — `set_task_status_with_log` replaced with the same graceful pause pattern  
3. **Behavioral test** — `test_fs_fs_abstract_pause_on_runner_error` passes, using an empty-description task to trigger the pre-MCP error and asserting the full pause + stack push flow