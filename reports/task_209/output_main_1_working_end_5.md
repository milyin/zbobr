Both fixes are implemented and all tests pass:

1. **Stage-title parsing** now rejects malformed model tokens with an error instead of silently dropping them — restoring strict parse semantics.

2. **Quota/rate-limit failures** now trigger provider exclusion: added `quota_failure: bool` to `ExecutorOutput`, a shared `detect_quota_failure()` helper in `tool_executor.rs` (scans for "rate limit", "too many requests", "quota exceeded", "usage limit", "account limit", "rate_limit_error"), used it in both claude and copilot executors, and updated `cli.rs` to propagate `quota_failure` as `connectivity_failure` for provider exclusion purposes.