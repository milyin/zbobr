# Test: validate_unknown_executor

**File:** `zbobr-api/src/config.rs` (in existing `#[cfg(test)] mod tests`)

**Why:** The review fix (ae85fd88) added executor name validation in `validate()` (lines 634-643) that checks executor strings against `[Tool::CLAUDE, Tool::COPILOT, Tool::MCP_TESTER]`. However, no test was added for this new validation path. The existing validate tests cover: valid config, unknown parent, no executor/no parent, and unknown provider ref — but not unknown executor.

**Test case:**
- Provider: `executor = Some("invalid_executor")`, no parent
- Call `config.validate()`
- Assert error message contains "unknown executor"

This ensures the fix actually catches typos like `executor = "claud"` that previously silently fell through to Claude.