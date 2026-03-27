## Rationale

The chosen approach uses git's `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_*`/`GIT_CONFIG_VALUE_*` environment variables to inject the `insteadOf` URL rewrite at process level. This is the most minimal change that addresses all three leakage vectors (command args, error logs, on-disk config) while keeping the same underlying git authentication mechanism.

**Why this approach over alternatives:**
- **vs. GIT_ASKPASS**: Requires creating temporary script files, more complex
- **vs. http.extraHeader**: Requires base64 encoding, less readable
- **vs. credential helper**: Requires external helper process, more moving parts
- **Env-based config override**: Native git feature, zero external dependencies, process-scoped (no persistence), clean error messages

**Key design decisions:**
- Added `_env` function variants rather than changing existing signatures, to minimize disruption to callers that don't need auth
- Legacy `insteadOf` entries are cleaned up from existing bare repos on first run
- The `commands.rs` standalone fetch is simply removed (rewrite-authors is idempotent, update_worktree handles fetching)

**3 files modified, 6 checklist items for the worker.**
