# Test Implementation Report

## Tests Added

Two new tests in `zbobr/src/init.rs` (`mod tests`):

### 1. `tester_prompt_excludes_formatting_linting_and_defers_to_separate_stage`
- Asserts `TESTER_PROMPT` does NOT contain "run formatting" (case-insensitive)
- Asserts `TESTER_PROMPT` does NOT contain "fix formatting" (case-insensitive)
- Asserts `TESTER_PROMPT` DOES contain "separate stage"

### 2. `linter_prompt_covers_formatting_and_linting_without_testing`
- Asserts `LINTER_PROMPT` contains "formatting" (core duty)
- Asserts `LINTER_PROMPT` contains "linting" (core duty)
- Asserts `LINTER_PROMPT` does NOT contain "run comprehensive test"

## Results

All 17 tests pass (`cargo test -p zbobr`).

## Commit

`7c8fa1e1` — test: verify TESTER_PROMPT and LINTER_PROMPT content separation
