# Test: `write_or_new` creates file at non-existing path

**File:** `zbobr/src/init.rs` (add to existing `mod tests` block)

**Test name:** `write_or_new_creates_new_file`

**Setup:**
- Create a temp directory with `tempfile::tempdir()`
- Construct a path that doesn't exist yet

**Action:**
- Call `write_or_new(&path, "new content", false).await`

**Assertions:**
- The file now exists and contains "new content"

**Why:** The "file doesn't exist" branch (line 101-103) is the base case. While not directly changed by the force flag, it completes the coverage of all `write_or_new` branches and ensures the refactoring didn't break file creation.