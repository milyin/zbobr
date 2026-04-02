# Tests for `validate()` — zbobr-api/src/config.rs

These tests validate the configuration consistency checks.

## Test cases

### 1. `validate_valid_config`
Create a valid config with providers referencing valid parents and tools referencing valid providers. Assert `validate()` returns Ok.

### 2. `validate_unknown_parent`
Create a provider that references a non-existent parent. Assert `validate()` returns error containing "unknown parent".

### 3. `validate_no_executor_no_parent`
Create a provider with neither executor nor parent set. Assert `validate()` returns error containing "neither 'executor' nor 'parent'".

### 4. `validate_tool_references_unknown_provider`
Create a tool entry that references a non-existent provider name. Assert `validate()` returns error containing "unknown provider".

## Location

Same `#[cfg(test)] mod tests` in `zbobr-api/src/config.rs`.

## Dependencies

Plain struct construction only.
