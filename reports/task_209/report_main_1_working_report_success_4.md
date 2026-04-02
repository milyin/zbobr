## Summary

Fixed two correctness gaps found in the last review.

### Fix 1: Stage-title parsing rejects malformed model tokens

**File:** `zbobr-api/src/context/stage_title.rs:168`

Changed `.parse::<Model>().ok()` to propagate an error when a second non-timestamp backtick token is present but `Model::from_str` fails. Previously, an invalid model token would be silently converted to `None`, causing data loss during parse/serialize round-trips. Now parsing fails with a clear error message.

### Fix 2: Quota/account-limit failures trigger provider exclusion

**Files changed:**
- `zbobr-api/src/tool_executor.rs`: Added `quota_failure: bool` field to `ExecutorOutput` and a shared `detect_quota_failure(&str) -> bool` helper that scans output (case-insensitive) for patterns: "rate limit", "too many requests", "quota exceeded", "usage limit", "account limit", "rate_limit_error"
- `zbobr-executor-claude/src/lib.rs`: Sets `quota_failure = !status.success() && detect_quota_failure(&output)`
- `zbobr-executor-copilot/src/lib.rs`: Same
- `zbobr-executor-mcp-tester/src/lib.rs`: Sets `quota_failure: false` (no quota concept for test tool)
- `zbobr-dispatcher/src/cli.rs`: In the `Ok(exit_ok: false, quota_failure)` arm, sets `connectivity_failure: quota_failure` so exhausted providers are temporarily excluded from rotation

### Test results

All 243 tests pass (0 failures).