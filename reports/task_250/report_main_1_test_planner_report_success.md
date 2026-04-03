## Test Plan: Separation of formatting/linting concerns in prompts

### Assessment

The existing 4 new tests thoroughly cover structural wiring (stage order, role assignment, tool config, model/priority contract). The remaining gap is **prompt content regression tests** that verify the core behavioral change: splitting formatting/linting responsibility between TESTER_PROMPT and LINTER_PROMPT.

### Tests to add

#### 1. TESTER_PROMPT excludes formatting/linting and defers to separate stage
- Assert TESTER_PROMPT does NOT contain removed instructions (e.g., "Run formatting/linting checks", "Formatting fixes are allowed")
- Assert TESTER_PROMPT DOES contain the deferral note ("separate stage" or equivalent)
- Prevents accidental re-introduction of formatting duties into the tester

#### 2. LINTER_PROMPT covers formatting and linting, not testing
- Assert LINTER_PROMPT contains "formatting" and "linting"
- Assert LINTER_PROMPT does NOT contain test-running instructions (e.g., "Run comprehensive test")
- Ensures the linter stays focused on its responsibility and doesn't overlap with the tester

### Files to modify
- `zbobr/src/init.rs` — add 2 tests in `mod tests`

### Verification
- `cargo test -p zbobr` — all tests (existing 15 + 2 new) should pass