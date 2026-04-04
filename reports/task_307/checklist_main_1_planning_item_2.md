In `init.rs`:
1. Add `force: bool` parameter to `init_workspace()` function signature.
2. Pass `force` to all `write_or_new()` calls (lines 55 and 70).
3. Update `write_or_new()` (line 81) to accept a `force: bool` parameter. When `force` is true and the file exists with different content, overwrite it in place (using `tokio::fs::write` to the original path) instead of writing to the `.new` path. Print a message like `"  overwrote {path}"` to distinguish from the normal write. The "unchanged" and "new file" code paths remain the same regardless of `force`.