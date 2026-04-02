## Problem

The `tool` field on `RoleDefinition` is currently `Option<String>`. The `resolve_tool_name()` function uses the precedence chain: stage.tool → role.tool → bail. Previously there was a global `tool` fallback on `ZbobrDispatcherConfig`, but it has been removed. This means:

1. If a role has no `tool` and the stage doesn't override it, `resolve_tool_name` fails at **runtime** — too late.
2. There are 11 compilation errors in the test suite: tests still reference the removed global `tool` field on `ZbobrDispatcherConfig`.

## Proposed Plan

### Step 1: Add validation that every role must have `tool` defined

In `validate_workflow_refs()` (on `ZbobrDispatcherConfig`), add a check that rejects roles where `tool` is `None`. This catches the misconfiguration at startup rather than at runtime. The error message should follow the existing pattern (e.g., "Role 'worker' has no tool defined").

### Step 2: Fix broken `resolve_tool_name` tests

The 4 tests that construct `ZbobrDispatcherConfig { tool: "global-tool" ... }` need updating:
- **`resolve_tool_name_stage_overrides`** and **`resolve_tool_name_falls_back_to_role`**: remove the non-existent `tool` field; these tests already have tool at stage/role level so they should pass.
- **`resolve_tool_name_falls_back_to_global`** and **`resolve_tool_name_no_role_falls_back_to_global`**: these tested the removed global fallback. Rewrite them to test the **error case** — i.e., that `resolve_tool_name` returns an error when neither stage nor role provides a tool.

### Step 3: Fix broken global-tool validation tests

The 3 tests (`validate_rejects_unknown_global_tool`, `validate_rejects_when_tools_empty`, `validate_passes_when_global_tool_exists`) reference the removed `config.tool` field. Remove or repurpose these tests since the global tool concept no longer exists.

### Step 4: Fix `validate_workflow_refs_passes_no_tool_refs` test

This test creates a role with `tool: None` and expects `validate_workflow_refs` to pass. After Step 1, this should be updated to expect failure, or the test should give the role a valid tool.

### Step 5: Add new test for the validation

Add a test that explicitly verifies a role without a `tool` is rejected by `validate_workflow_refs`.

## Analog

The closest analog for the new validation check is the existing pattern in `validate_workflow_refs()` itself, which already checks that role/stage tool references point to known tool names. The new check simply adds a "tool must be present" guard before the "tool must exist in tools map" guard.

## Key Constraint

No changes to the `RoleDefinition` struct type itself are needed (keep `tool` as `Option<String>` for serde flexibility). The enforcement is purely at the validation layer.