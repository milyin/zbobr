# Fix: Integrate `--logs` into GlobalArgs for global CLI hoisting

## Problem
The `--logs` flag was added only to the `Cli` struct in `zbobr/src/main.rs`. The global CLI parsing mechanism in `zbobr-dispatcher/src/cli.rs` (`parse_cli` + `GlobalArgs`) didn't know about it, so `--logs` placed after a subcommand (e.g. `zbobr task process --logs`) was rejected as an unexpected argument.

## Root Cause
`parse_cli` uses `GlobalArgs` to determine which flags should be hoisted from after the subcommand to before it. Since `--logs` wasn't in `GlobalArgs`, it wasn't hoisted and clap couldn't parse it in that position.

## Fix
Added the `logs: bool` field with `#[arg(long)]` to `GlobalArgs` in `zbobr-dispatcher/src/cli.rs`. This is a one-line structural change (plus doc comment) that makes the hoisting logic aware of `--logs`.

The `Cli` struct in `main.rs` retains its own `logs` field for actual value access — this is consistent with how other global args work (e.g. `config_file` appears in both `GlobalArgs` and `Cli`).

## Changes
- `zbobr-dispatcher/src/cli.rs`: Added `logs: bool` field to `GlobalArgs` struct

## Verification
- `cargo build` succeeds
- `cargo test --package zbobr` — all 12 tests pass
- `zbobr --logs task process --help` — parses correctly
- `zbobr task process --logs --help` — parses correctly (was previously rejected)
- `zbobr --help` — shows `--logs` flag with description