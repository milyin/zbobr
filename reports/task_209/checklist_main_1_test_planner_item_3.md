# Tests for `resolve_tool_name()` — zbobr-api/src/config.rs

These tests validate the tool name resolution precedence: stage.tool → role.tool → dispatcher.tool.

## Test cases

### 1. `resolve_tool_name_stage_overrides`
Create a `StageDefinition` with `tool: Some("stage-tool")`, a role with `tool: Some("role-tool")`, and dispatcher with `tool: "global-tool"`. Assert `resolve_tool_name()` returns "stage-tool".

### 2. `resolve_tool_name_falls_back_to_role`
Create a `StageDefinition` with `tool: None` and a `role` in the workflow with `tool: Some("role-tool")`, dispatcher `tool: "global-tool"`. Assert `resolve_tool_name()` returns "role-tool".

### 3. `resolve_tool_name_falls_back_to_global`
Create a `StageDefinition` with `tool: None`, a role with `tool: None`, dispatcher `tool: "global-tool"`. Assert `resolve_tool_name()` returns "global-tool".

### 4. `resolve_tool_name_no_role_falls_back_to_global`
Create a `StageDefinition` with `tool: None` and no role (or a role that doesn't exist in workflow). Assert fallback to global.

## Location

Same `#[cfg(test)] mod tests` in `zbobr-api/src/config.rs`.

## Dependencies

Requires constructing `StageDefinition`, `WorkflowConfig` (with roles), and `ZbobrDispatcherConfig`. All are plain structs/maps, no mocking needed.
