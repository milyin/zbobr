## Goal
Remove the old `Secret` implementation from `zbobr-api` and delegate to `zbobr-utility` instead.

## Changes

1. **Delete** `zbobr-api/src/secret.rs` entirely.

2. **In `zbobr-api/src/lib.rs`:**
   - Remove `pub mod secret;`
   - Change `pub use secret::Secret;` to `pub use zbobr_utility::Secret;`
   - No `extern crate` or dependency changes needed — `zbobr-utility` is already a dependency of `zbobr-api`.

3. **In `zbobr-api/src/config.rs`** (the Default impl for `ZbobrDispatcherConfig`):
   - Replace `Secret::Value("not-configured".to_string())` with `Secret::value("not-configured")` (using the new constructor)

## No Cargo.toml changes needed
`zbobr-utility` is already listed as a workspace dependency in `zbobr-api/Cargo.toml`.
