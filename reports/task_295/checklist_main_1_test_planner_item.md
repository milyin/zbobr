# Cli `--logs` parsing tests

**File:** `zbobr/src/main.rs`, test module starting at line 112.

## Test 1: `logs_flag_defaults_to_false`
Parse `["zbobr", "task", "process", "--select"]` via `Cli::try_parse_from`. Assert `cli.logs == false`. Validates that logs are suppressed by default.

## Test 2: `logs_flag_parses_when_present`
Parse `["zbobr", "--logs", "task", "process", "--select"]` via `Cli::try_parse_from`. Assert `cli.logs == true`. Validates that the flag activates logging.

Both tests use the real `Cli` struct (not `TestCli`) since they need access to the `logs` field. Follow the existing test style in the module.