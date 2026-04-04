# GlobalArgs `--logs` hoisting contract test

**File:** `zbobr-dispatcher/src/cli.rs`, test module starting at line 1922.

## Test: `global_args_includes_logs_flag`
Use `GlobalArgs::augment_args(clap::Command::new(""))` to build the command, then find the argument with long name `"logs"`. Assert:
1. The argument exists (preventing removal regression).
2. Its action is `SetTrue` (boolean flag, not value-taking — ensuring the hoisting logic in `parse_cli` handles it correctly as a no-value flag).

**Why this matters:** During implementation, `--logs` was initially missing from `GlobalArgs`, causing `zbobr task process --logs` to fail. This test guards against that exact regression.