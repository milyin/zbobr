## Changes Made

### 1. Added role tool validation (`zbobr-api/src/config.rs`)
In `validate_workflow_refs()`, replaced the `if let Some(ref tool)` pattern with a `let Some(ref tool) else { bail }` guard that rejects any role where `tool` is `None`. Error message: `"Role '{}' has no tool defined"`.

### 2. Updated test: `validate_workflow_refs_passes_no_tool_refs` → `validate_workflow_refs_rejects_role_without_tool`
Renamed and changed assertion from `is_ok()` to `unwrap_err()` + message check, verifying the new validation rejects tool-less roles.

### 3. Removed 3 obsolete global-tool validation tests
Deleted `validate_rejects_unknown_global_tool`, `validate_rejects_when_tools_empty`, and `validate_passes_when_global_tool_exists` — all referenced the removed `config.tool` field.

### 4. Fixed 4 `resolve_tool_name` tests
- `resolve_tool_name_stage_overrides` and `resolve_tool_name_falls_back_to_role`: removed `tool` field from config construction, added `.unwrap()` on result.
- `resolve_tool_name_falls_back_to_global` → `resolve_tool_name_errors_when_no_tool`: rewritten to test error case.
- `resolve_tool_name_no_role_falls_back_to_global` → `resolve_tool_name_errors_when_no_role`: rewritten to test error case.

### 5. Fixed 2 integration tests (`zbobr-dispatcher/tests/mcp_integration/env.rs`)
Removed `tool: "mcp-tester".to_string()` from both `init_fs_fs` and `init_github_github` config construction (the tool is already set on role definitions in test workflow configs).

### Verification
All 99 zbobr-api tests pass. All test binaries compile successfully.