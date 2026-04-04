## Plan: Suppress logs by default

### Approach
Minimal change to `zbobr/src/main.rs` only:
1. Add `--logs` boolean flag to `Cli` struct
2. Move tracing subscriber init after CLI parsing
3. Conditionally set filter: `"off"` by default, `RUST_LOG`/`"info"` when `--logs` is passed

### Key decisions
- Single file change keeps it simple
- Flag is top-level (not per-subcommand) since logging is a global concern
- Preserves existing `RUST_LOG` env var support when `--logs` is active
- No analog needed — this is a straightforward clap flag addition following existing patterns in the `Cli` struct