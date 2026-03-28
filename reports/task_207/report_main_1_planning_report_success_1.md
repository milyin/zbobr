## Plan: Remove Backward Compatibility Parsing

### Approach
All backward compatibility code is confined to `zbobr-api/src/context/stage_title.rs`. The changes are surgical and confined to that single file.

### What to remove
- Module doc comment lines documenting the old `<sub>` timestamp format
- `parse_trailing_timestamp_sub` helper function (only used for old format)
- The `else` branch in `FromStr::from_str` that falls back to the old format — replace with an error return if no backtick timestamp is found
- Two old-format tests: `parse_old_format_with_link` and `parse_old_format_plain_timestamp`

### Why this approach
The old format (`<sub>timestamp</sub>`) was only needed to parse data written before the new backtick format was introduced. Since the task says to remove backward compatibility, the parser should simply reject old-format titles. No other files reference the old-format parsing path.

### Verification
Run `cargo test -p zbobr-api` after changes to confirm remaining tests pass.