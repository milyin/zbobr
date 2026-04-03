# Test Coverage Added for Linting Stage

The reviewer identified that the new linting workflow wiring introduced in the previous session lacked test coverage. Added 4 new tests in `zbobr/src/init.rs`:

1. **`default_workflow_has_linting_stage_before_testing`** — verifies `linting` exists in main pipeline and appears before `testing` by index position
2. **`default_workflow_linting_stage_uses_linter_role`** — verifies `linting` stage has `role = "linter"`
3. **`default_workflow_linter_role_uses_drudge_tool_and_linter_prompt`** — verifies `linter` role has `tool = "drudge"` and `prompt = "linter.md"`
4. **`default_config_toml_has_drudge_tool`** — verifies `default_config_toml()` includes a `drudge` tool entry

All 15 tests pass (11 existing + 4 new). Committed as `599b97ae`.