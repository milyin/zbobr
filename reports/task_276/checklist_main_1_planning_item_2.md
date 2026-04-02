## What

In `ZbobrDispatcherConfig::validate_workflow_refs()` (in `zbobr-api/src/config.rs`), add a check at the beginning of the roles iteration that rejects any `RoleDefinition` where `tool` is `None`.

## Why

Currently, if a role has no `tool` and the stage doesn't override it, `resolve_tool_name` fails at **runtime** — too late. This validation catches the misconfiguration at startup time, before any tasks are processed.

## How (pattern to follow)

Follow the existing pattern in `validate_workflow_refs` itself. The function already iterates over roles and checks `if let Some(ref tool) = role_def.tool`. Add a guard **before** that check: if `role_def.tool.is_none()`, bail with an error message like `"Role '{}' has no tool defined"` (following the existing error message style in the function).

Keep `RoleDefinition.tool` as `Option<String>` — no struct changes needed. Enforcement is purely at the validation layer.