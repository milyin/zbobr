The plan is ready for review. It's a minimal single-file change to `zbobr/src/main.rs`:

1. Add `--logs` boolean flag to the `Cli` struct
2. Move tracing subscriber initialization after CLI parsing
3. Default filter becomes `"off"` (silent) unless `--logs` is passed, in which case the current behavior (`RUST_LOG` or `"info"`) is preserved