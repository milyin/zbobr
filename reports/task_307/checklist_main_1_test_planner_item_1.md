# Test: `write_or_new` default creates `.new` sibling

**File:** `zbobr/src/init.rs` (add to existing `mod tests` block)

**Test name:** `write_or_new_no_force_creates_dot_new_file`

**Setup:**
- Create a temp directory with `tempfile::tempdir()`
- Write a file `example.toml` with initial content "old content"

**Action:**
- Call `write_or_new(&path, "new content", false).await`

**Assertions:**
- The original file still contains "old content" (untouched)
- A sibling file `example.toml.new` exists and contains "new content"

**Why:** This is the counterpart branch to `force=true`. Testing both sides of the conditional ensures the flag actually controls behavior. This path existed before but was never tested; it's now critical to verify the refactoring didn't break it.