# Test Group: Clap parse-level tests for `task process --select`

## Location
`zbobr/src/commands.rs` — add a `#[cfg(test)]` module at the bottom of the file.

## Rationale
The `conflicts_with = "select"` annotation on `task` was the subject of multiple review cycles and was the central correctness concern of this feature. There are currently **no** Clap parse tests anywhere in the `zbobr` crate. Adding them here establishes coverage for this contract.

Use `Cli::try_parse_from(["zbobr", "task", "process", ...])` (Clap's `Parser::try_parse_from`) to test parsing without running the async handler.

## Tests to add

### 1. `task_process_select_flag_parses_without_task_id`
```rust
let cli = Cli::try_parse_from(["zbobr", "task", "process", "--select"]).unwrap();
// Extract inner subcommand and assert: select == true, task == None
```

### 2. `task_process_explicit_id_parses_without_select`
```rust
let cli = Cli::try_parse_from(["zbobr", "task", "process", "42"]).unwrap();
// Assert: task == Some(42), select == false
```

### 3. `task_process_select_and_task_id_together_is_rejected`
```rust
let result = Cli::try_parse_from(["zbobr", "task", "process", "42", "--select"]);
assert!(result.is_err(), "should fail: task and --select are mutually exclusive");
```

## Notes
- The `Cli` struct and `TaskSubcommand` are already in scope within the `commands.rs` file, so no new imports are needed.
- These tests are pure synchronous unit tests — no async runtime or backend mocking required.
- They directly verify the contract that the code review flagged multiple times.
