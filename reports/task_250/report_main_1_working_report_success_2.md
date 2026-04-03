## Summary

The reviewer found that the `default_config_toml_has_drudge_tool` test only verified the tool existed and was non-empty, but did not assert the exact model/provider/priority contract required by the task.

## Change Made

Updated `default_config_toml_has_drudge_tool` in `zbobr/src/init.rs` to assert:
1. Exactly 2 entries in the drudge tool
2. Primary entry: provider = "copilot", model = "gpt-5-mini", priority = None
3. Backup entry: provider = "claude", model = "claude-haiku-4.5", priority = Some(0)

This ensures a future accidental model change (e.g., switching to a more expensive model) would be caught by the test suite.

## Result

All 15 tests pass.