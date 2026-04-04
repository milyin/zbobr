The test plan proposes **3 tests across 2 files**:

1. **`zbobr/src/main.rs`** — Two `Cli` parsing tests: `logs_flag_defaults_to_false` and `logs_flag_parses_when_present`
2. **`zbobr-dispatcher/src/cli.rs`** — One hoisting contract test: `global_args_includes_logs_flag` (guards against the exact regression from ctx_rec_6)