In `zbobr-api/src/context/stage_title.rs`, remove all backward-compatibility support for the old `<sub>` timestamp format:

1. **Module-level doc comment**: Remove the "For backwards compatibility" paragraph and the two old-format examples (the `<sub>[YYYY-MM-DD ...](url)</sub>` and `<sub>YYYY-MM-DD ...</sub>` lines).

2. **`parse_trailing_timestamp_sub` helper function**: Delete it entirely. It was only used for the old format.

3. **`FromStr` implementation**: Simplify the timestamp resolution block. After the backtick loop, if `timestamp_from_backtick` is `None`, return an error (e.g. `anyhow::bail!("Missing backtick timestamp")`) instead of falling back to `parse_trailing_timestamp_sub`. Remove the `else` branch and the `(ts, prompt_link, output_link)` tuple assignment — just directly extract the new-format links after the backtick timestamp.

4. **Tests**: Remove `parse_old_format_with_link` and `parse_old_format_plain_timestamp` test cases, as the old format is no longer supported.

No changes are needed in other files. After editing, run `cargo test -p zbobr-api` to confirm the remaining tests pass.