# Fix: error formatting and test update

## Issues addressed (from review ctx_rec_8)

1. **Failing unit test**: `assert_eq!(task.error.as_deref(), Some("oops"))` was failing because `RoleSession::set_error` now formats errors as `"❌ YYYY-MM-DD HH:MM:SS +ZZZZ oops"`. Fixed by changing the assertion to check that the error starts with `❌` and contains `"oops"`.

2. **API bypass**: `TaskMut::set_error` default in `zbobr-api/src/backend.rs` set the raw error string without formatting. Added the same ❌ + timestamp formatting (using `chrono::Utc::now()` since the trait has no config/timezone access) so any code calling `set_error` through the `TaskMut` trait also gets formatted errors.

## Changes

- `zbobr-api/src/backend.rs`: Added ❌ + UTC timestamp formatting to `TaskMut::set_error` default implementation
- `zbobr-dispatcher/src/task.rs`: Updated test assertion from exact string match to checking prefix ❌ + contains original message

## Tests

All tests pass (0 failures).