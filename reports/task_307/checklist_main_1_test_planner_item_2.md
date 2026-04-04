# Test: `write_or_new` skips identical content

**File:** `zbobr/src/init.rs` (add to existing `mod tests` block)

**Test name:** `write_or_new_skips_identical_content`

**Setup:**
- Create a temp directory with `tempfile::tempdir()`
- Write a file with content "same content"

**Action:**
- Call `write_or_new(&path, "same content", true).await`

**Assertions:**
- The file still contains "same content"
- No `.new` sibling file was created
- Function returns `Ok(())`

**Why:** The identical-content early return at line 86-88 is the first branch checked, before the force flag is ever consulted. This test verifies the "unchanged" path works correctly and that `force=true` doesn't cause unnecessary overwrites of identical files.