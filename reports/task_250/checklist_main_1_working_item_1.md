The reviewer found that the current test for the drudge tool only checks existence, not the exact model/provider/priority configuration required by the task:
- copilot model: gpt-5-mini (primary, no explicit priority)
- claude model: claude-haiku-4.5 with priority = 0 (backup)

Update the test `default_config_toml_has_drudge_tool` in zbobr/src/init.rs to assert the full drudge tool entry configuration including providers, models, and backup priority.