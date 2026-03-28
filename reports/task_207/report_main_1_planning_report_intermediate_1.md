# Plan: Remove Backward Compatibility Code

## Background

The previous working sessions added backward compatibility to parse the old stage title format (`<sub>` timestamp) while writing the new format (backtick timestamp). The user now wants that compatibility removed.

## Changes Required (all in `zbobr-api/src/context/stage_title.rs`)

1. **Module doc comment** — remove the "For backwards compatibility" paragraph that documents the two old formats.

2. **`FromStr` implementation** — remove the `else` branch that calls `parse_trailing_timestamp_sub` when no backtick timestamp is found. Replace it with an error (the parser should require a backtick timestamp now). The "New format:" comment on the remaining branch becomes unnecessary and should be cleaned up.

3. **`parse_trailing_timestamp_sub` function** — delete the entire function; it only served the old format path.

4. **Tests** — delete `parse_old_format_with_link` and `parse_old_format_plain_timestamp` tests, which only verify the removed behavior.

## No other files need changes

The executor, dispatcher, context/mod.rs, and task.rs changes from earlier sessions are all part of the new format feature itself, not backward compatibility shims.
