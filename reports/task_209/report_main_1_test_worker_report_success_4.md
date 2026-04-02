## Test Implementation Results

### Commit: a7893518

### Tests added

**`detect_quota_failure()` in `zbobr-api/src/tool_executor.rs` (7 tests)**
1. `detect_quota_failure_rate_limit` — "rate limit" pattern matches ✅
2. `detect_quota_failure_too_many_requests` — "too many requests" pattern matches ✅
3. `detect_quota_failure_quota_exceeded` — "quota exceeded" pattern matches ✅
4. `detect_quota_failure_usage_limit` — "Usage Limit" mixed case matches (case-insensitivity) ✅
5. `detect_quota_failure_account_limit` — "account limit" pattern matches ✅
6. `detect_quota_failure_rate_limit_error` — "rate_limit_error" pattern matches ✅
7. `detect_quota_failure_no_match` — unrelated output returns false ✅

**Stage-title parsing in `zbobr-api/src/context/stage_title.rs` (2 tests)**
1. `parse_rejects_malformed_model_token` — backtick model with whitespace returns Err with "Invalid model token" ✅
2. `parse_accepts_valid_model_token` — valid tool+model backticks parse correctly ✅

### Test run output
All 15 tests in the filtered set passed (9 new + 6 existing stage_title tests).