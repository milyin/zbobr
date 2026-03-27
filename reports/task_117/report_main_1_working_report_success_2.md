## Review Fix Summary

### Issue: Token leakage in log output (ctx_rec_16)
**Problem:** `cleanup_legacy_token_config()` logged git config keys directly. Keys like `url.https://x-access-token:TOKEN@github.com/.insteadOf` would leak the token into application logs.

**Fix:** Added credential redaction before logging. When the key contains a `://...@` pattern (indicating embedded credentials), the portion between `://` and `@` is replaced with `[REDACTED]`. For example:
- `url.https://x-access-token:ghp_secret@github.com/.insteadOf` → `url.https://[REDACTED]@github.com/.insteadOf`

If no credentials pattern is found, the key is logged as-is (safe case).

**File:** `zbobr-repo-backend-github/src/github.rs` lines 294-310

### Issue: base64 dependency not centralized (ctx_rec_17)
**Problem:** `base64 = "0.22"` was specified directly in `zbobr-repo-backend-github/Cargo.toml` instead of using workspace dependency. Same issue pre-existed in `zbobr-task-backend-github/Cargo.toml`.

**Fix:** Added `base64 = "0.22"` to `[workspace.dependencies]` in root `Cargo.toml`. Updated both crate Cargo.toml files to use `base64 = { workspace = true }`.

### Verification
- `cargo build`: success
- `cargo test`: all tests pass (112 tests)