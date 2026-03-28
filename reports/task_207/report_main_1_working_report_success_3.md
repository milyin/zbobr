## Summary

Removed all backward compatibility code for parsing old-format stage titles from `stage_title.rs`.

### Changes made

**`zbobr-api/src/context/stage_title.rs`:**
- Removed documentation comment describing the old `<sub>` timestamp formats
- Replaced the `if let Some(ts) = timestamp_from_backtick { ... } else { ... }` branching in `FromStr` with a direct `.ok_or_else(...)` error if no backtick timestamp is found
- Removed the `parse_trailing_timestamp_sub` function entirely
- Removed two old-format test cases: `parse_old_format_with_link` and `parse_old_format_plain_timestamp`

**`zbobr-api/src/context/mod.rs`:**
- Updated two test fixtures that used the old `<sub>2024-01-01 00:00:00 +0000</sub>` format to use the new `` `2024-01-01 00:00:00 +0000` `` backtick format

### Result
All 119 tests pass. The parser now strictly requires the new backtick-timestamp format and returns an error for old-format inputs.
