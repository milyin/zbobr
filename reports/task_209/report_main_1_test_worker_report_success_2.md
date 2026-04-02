# Test Implementation Report

## Commit: bc99329b

All 12 new tests implemented across 2 files, all passing (92 total tests in zbobr-api).

### validate() global tool check — zbobr-api/src/config.rs (3 tests)
1. `validate_rejects_unknown_global_tool` — config with `tool = "nonexistent"` and tools map containing `"smart"` → error contains "not defined in [tools]"
2. `validate_passes_when_tools_empty` — config with `tool = "anything"` and empty tools map → succeeds (backward compat guard)
3. `validate_passes_when_global_tool_exists` — config with `tool = "smart"` matching tools map → succeeds

### validate_workflow_refs() — zbobr-api/src/config.rs (4 tests)
1. `validate_workflow_refs_rejects_unknown_role_tool` — role with `tool = "nonexistent"` → error contains "Role 'worker' references unknown tool"
2. `validate_workflow_refs_rejects_unknown_stage_tool` — stage with `tool = "bad"` → error contains "references unknown tool"
3. `validate_workflow_refs_passes_valid_refs` — role and stage both referencing existing tool "smart" → succeeds
4. `validate_workflow_refs_passes_no_tool_refs` — role and stage both with `tool = None` → succeeds

### Model::try_new() — zbobr-api/src/task.rs (5 tests)
1. `model_try_new_valid` — "claude-opus-4.6" → Ok, inner string matches
2. `model_try_new_rejects_space` — "claude opus" → Err containing "whitespace"
3. `model_try_new_rejects_tab` — "model\there" → Err
4. `model_from_str_rejects_whitespace` — "bad model".parse::<Model>() → Err (verifies FromStr delegates to try_new)
5. `model_deserialize_rejects_whitespace` — TOML deserialization of "bad model" into Model → Err (verifies Deserialize delegates to try_new)