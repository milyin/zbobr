## Test: LINTER_PROMPT content covers formatting and linting

**File:** `zbobr/src/init.rs` (in `mod tests`)

**What to verify:**
1. `LINTER_PROMPT` contains "formatting" (core duty)
2. `LINTER_PROMPT` contains "linting" (core duty)
3. `LINTER_PROMPT` does NOT contain "Run comprehensive test" or similar testing instructions (separation of concerns — linter should not run tests)

**Why:** Ensures the linter prompt actually covers its intended responsibility (formatting/linting) and doesn't overlap with the tester's responsibility (running tests).

**Pattern:** Content assertions on prompt constant, same as existing prompt tests.