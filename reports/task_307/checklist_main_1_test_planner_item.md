# Test: `write_or_new` force overwrites differing file

**File:** `zbobr/src/init.rs` (add to existing `mod tests` block)

**Test name:** `write_or_new_force_overwrites_existing_file`

**Setup:**
- Create a temp directory with `tempfile::tempdir()`
- Write a file with initial content "old content"

**Action:**
- Call `write_or_new(&path, "new content", true).await`

**Assertions:**
- The original file path contains "new content" (overwritten in place)
- No `.new` sibling file was created

**Why:** This is the core new behavior introduced by the `--force` flag. The `force=true` branch at line 90-92 of `init.rs` must be covered.