All done. The implementation was already complete — this planning session confirms all three items were properly implemented:

- Both `runner.run()` error call sites in `cli.rs` now call `set_pause_with_status_and_signal` instead of propagating/logging-only
- Behavioral test `run_pause_on_runner_error` verifies the full pause-then-convert flow using an empty-description task as the error trigger