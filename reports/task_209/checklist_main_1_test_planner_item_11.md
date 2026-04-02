## Unit tests for `detect_quota_failure()` in `zbobr-api/src/tool_executor.rs`

### Location
Add a `#[cfg(test)] mod tests { ... }` block at the bottom of `zbobr-api/src/tool_executor.rs`.

### Tests to add (7 tests)

1. **`detect_quota_failure_rate_limit`** — Input containing "rate limit" returns `true`.
2. **`detect_quota_failure_too_many_requests`** — Input containing "too many requests" returns `true`.
3. **`detect_quota_failure_quota_exceeded`** — Input containing "quota exceeded" returns `true`.
4. **`detect_quota_failure_usage_limit`** — Input containing "Usage Limit" (mixed case) returns `true` — verifies case-insensitivity.
5. **`detect_quota_failure_account_limit`** — Input containing "account limit" returns `true`.
6. **`detect_quota_failure_rate_limit_error`** — Input containing "rate_limit_error" returns `true`.
7. **`detect_quota_failure_no_match`** — Input with no quota-related patterns (e.g. "Command failed with exit code 1") returns `false`.

### Rationale
`detect_quota_failure` is a public pure function with multiple string-matching branches. Each branch should be tested independently to prevent regressions if patterns are added/removed. The case-insensitivity behavior is important because executor output formatting varies between providers.