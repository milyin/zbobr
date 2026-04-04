Test plan complete. I identified 4 tests needed for `write_or_new` in `zbobr/src/init.rs`, covering all 4 branches of the function:

1. **Force overwrite** — `force=true` overwrites existing file with different content
2. **Default .new fallback** — `force=false` creates `.new` sibling, leaves original intact
3. **Identical content skip** — unchanged files are skipped regardless of force flag
4. **New file creation** — non-existing path creates the file

All tests are behavioral (asserting file system outcomes), use `tempfile::tempdir()` for isolation, and belong in the existing `mod tests` block as `#[tokio::test]` async tests.