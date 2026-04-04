# Test Plan for `--logs` Flag

## Summary
3 new tests needed across 2 files:

### `zbobr/src/main.rs` (2 tests)
- `logs_flag_defaults_to_false` — verifies `--logs` defaults to false
- `logs_flag_parses_when_present` — verifies `--logs` sets to true

### `zbobr-dispatcher/src/cli.rs` (1 test)
- `global_args_includes_logs_flag` — verifies `GlobalArgs` declares `--logs` as a boolean flag, preventing regression of the hoisting bug that was already caught during review (ctx_rec_6)

## Rationale
- The tracing filter logic ("off" vs "info") is internal to `main()` and not practical to unit test.
- The most valuable test is the `GlobalArgs` contract test, which guards against the exact regression that already occurred.
- The `Cli` parsing tests follow existing test patterns and validate the flag integration.