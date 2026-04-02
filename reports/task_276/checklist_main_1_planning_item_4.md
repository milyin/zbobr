## What

Fix the 4 `resolve_tool_name_*` test functions in `zbobr-api/src/config.rs` that reference a `tool` field on `ZbobrDispatcherConfig` which no longer exists.

## Tests to fix

1. **`resolve_tool_name_stage_overrides`** — Remove the `tool` field from the config construction. This test already has tool at the stage level, so it should work once the non-existent field is removed.

2. **`resolve_tool_name_falls_back_to_role`** — Remove the `tool` field from the config construction. This test already has tool at the role level, so it should work once the non-existent field is removed.

3. **`resolve_tool_name_falls_back_to_global`** — This tested the removed global fallback. Rewrite to test the **error case**: when neither stage nor role provides a tool, `resolve_tool_name` should return an error. Use `.is_err()` assertion.

4. **`resolve_tool_name_no_role_falls_back_to_global`** — Same as above: rewrite to test error case when no role exists for the stage and no stage tool is set. Use `.is_err()` assertion.

## Pattern

Also fix the `==` comparison errors — `resolve_tool_name` returns `Result<String>`, so use `.unwrap()` before comparing, or use `assert_eq!(result.unwrap(), expected)`.