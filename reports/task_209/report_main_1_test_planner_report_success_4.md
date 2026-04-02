## Test Plan for Latest Fix (commit e5d95307)

### Summary

The latest commit introduced two new behaviors with **zero existing test coverage**:

1. **`detect_quota_failure()` function** — A new public pure function in `zbobr-api/src/tool_executor.rs` that pattern-matches 6 quota/rate-limit signatures in executor output. Currently untested.

2. **Stage-title parsing now rejects malformed model tokens** — Changed from `.ok()` (silent drop) to error propagation in `zbobr-api/src/context/stage_title.rs:168`. No test exercises the error path.

### Test Groups

#### Group 1: `detect_quota_failure()` — 7 tests (ctx_rec_47)
- One test per pattern: "rate limit", "too many requests", "quota exceeded", "usage limit", "account limit", "rate_limit_error"
- One case-insensitivity test (mixed-case input)
- One negative test (no matching pattern → `false`)
- Location: new `#[cfg(test)] mod tests` in `zbobr-api/src/tool_executor.rs`

#### Group 2: Stage-title malformed model rejection — 2 tests (ctx_rec_48)
- Rejects stage title with invalid model backtick token (whitespace in model)
- Accepts stage title with valid model token (positive control)
- Location: existing test module in `zbobr-api/src/context/stage_title.rs`

### What does NOT need new tests

- **`quota_failure` field wiring in executors** (Claude, Copilot, MCP-tester): These are thin integration points that call `detect_quota_failure` — testing the function itself is sufficient. Integration testing would require process mocking.
- **`cli.rs` dispatcher wiring** (`connectivity_failure: quota_failure`): Simple field pass-through, covered by the existing `validated()`/`select_provider` integration tests at the dispatcher level.

### Total: 9 new tests across 2 files