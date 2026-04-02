All work is complete. The implementation:

1. **Fixed `process_task()`** — catches `runner.run()` error and pauses gracefully
2. **Fixed `run_manager_loop()`** — replaced raw status set with proper pause handling  
3. **Added behavioral test** `run_pause_on_runner_error` — verifies the full pause-on-error flow using an empty-description task to trigger a pre-MCP error

All tests pass.