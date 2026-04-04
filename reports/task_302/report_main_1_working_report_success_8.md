# Multi-Config Merge Test Implementation — Complete

## What was done

Added 8 new tests to `zbobr-api/src/config.rs` in a single commit (`92217707`), organized into three groups:

### 1. Option<Vec<T>> TOML Deserialization Round-Trip Tests (6 tests)
- `role_mcp_missing_deserializes_as_none` — missing `mcp` field → `None`
- `role_mcp_empty_list_deserializes_as_some_empty` — `mcp = []` → `Some(vec![])`
- `role_mcp_populated_list_deserializes_as_some_with_entries` — populated list → `Some(vec![...])`
- `stage_prompts_missing_deserializes_as_none` — missing `prompts` field → `None`
- `stage_prompts_empty_list_deserializes_as_some_empty` — `prompts = []` → `Some(vec![])`
- `stage_prompts_populated_list_deserializes_as_some_with_entries` — populated list → `Some(vec![...])`

These lock down the serde contract that the `.or()` merge semantics depend on.

### 2. Tools Map Merge Test (1 test)
- `dispatcher_toml_merge_tools_key_wise_with_wholesale_list_replacement` — verifies `IndexMap<String, Vec<ToolEntry>>` key-wise merge where per-key Vec lists are replaced wholesale (not merged element-by-element). Tests overlay replacing, base surviving, and new keys being added.

### 3. End-to-End Multi-Config Merge Test (1 test)
- `workflow_toml_end_to_end_merge_from_toml_strings` — full pipeline: parse two TOML strings → deserialize to `WorkflowToml` → merge → verify Vec fields (mcp override, prompt inheritance, prompts clearing, stage role inheritance, untouched stages surviving).

## Test results
All 126 tests in zbobr-api pass (0 failures).