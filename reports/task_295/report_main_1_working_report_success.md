## Implementation Summary

### Change: `zbobr/src/main.rs`

1. **Added `--logs` boolean flag** to the `Cli` struct with help text "Enable log output to stderr". Placed before `config_file` field as a top-level option.

2. **Reordered `main()`**: CLI parsing now happens before tracing subscriber initialization, so the `--logs` flag value is available when configuring the logger.

3. **Conditional tracing filter**:
   - Default (`--logs` not passed): filter is `"off"` — no log output
   - With `--logs`: uses `RUST_LOG` env var if set, otherwise falls back to `"info"` (preserving previous default behavior)

### Verification
- `cargo build -p zbobr` — compiles successfully
- `zbobr --help` — shows `--logs` flag with description "Enable log output to stderr"
- Default behavior suppresses all log output
- `--logs` enables info-level logging with RUST_LOG override support

### Commit
- `193537fb` — feat: add --logs flag to suppress log output by default