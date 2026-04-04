## Plan: Suppress logs by default, add `--logs` flag

### Context
Currently `zbobr` outputs tracing logs at `info` level to stdout by default, cluttering normal CLI output. The task is to suppress all log output by default and only show logs when `--logs` is explicitly passed.

### Current State
- Tracing subscriber initialized in `zbobr/src/main.rs:59-63` before CLI parsing (line 65)
- Default filter: `RUST_LOG` env var, falling back to `"info"`
- No existing verbosity flags

### Approach
1. **Add `--logs` flag to `Cli` struct** in `zbobr/src/main.rs` as a top-level boolean flag
2. **Move tracing init after CLI parsing** — reorder so CLI is parsed first, then tracing is configured based on the flag
3. **Conditional filter**: without `--logs` → filter `"off"` (silent); with `--logs` → current behavior (`RUST_LOG` or `"info"`)

### File to modify
- `zbobr/src/main.rs`

### Verification
- `zbobr --help` shows `--logs`
- Commands without `--logs` produce no log output
- Commands with `--logs` show info-level logs
- `--logs` + `RUST_LOG=debug` shows debug logs