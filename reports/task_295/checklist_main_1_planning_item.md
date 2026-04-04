## What to change

Modify `zbobr/src/main.rs` to add a `--logs` boolean flag and conditionally initialize tracing.

### 1. Add `--logs` flag to the `Cli` struct

Add a top-level `--logs` boolean field to the `Cli` struct (before the `config_file` field). Use clap's `#[arg(long)]` attribute. The flag should default to `false`. Add a help string like "Enable log output to stderr".

### 2. Reorder main() — parse CLI first, then init tracing

Currently tracing is initialized at lines 59-63 *before* CLI parsing at line 65. Swap the order:
- Parse CLI first (the `zbobr_dispatcher::parse_cli` call)
- Then conditionally initialize tracing based on `cli.logs`

### 3. Conditional tracing filter

- If `cli.logs` is `true`: use current behavior — `RUST_LOG` env var, falling back to `"info"`
- If `cli.logs` is `false` (default): use filter `"off"` to suppress all log output

The tracing subscriber init code stays the same structurally, just the filter changes based on the flag.

### Why

Currently all commands emit info-level tracing logs to stdout by default, which clutters normal CLI output. Users should only see logs when they explicitly opt in via `--logs`.