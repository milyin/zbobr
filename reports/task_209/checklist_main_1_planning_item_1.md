# Task types refactor (zbobr-api/src/task.rs)

## What to change

Replace `Tool` enum (Copilot, Claude, McpTester) with a string newtype:
```
pub struct Tool(pub String);
impl Tool {
    pub const CLAUDE: &'static str = "claude";
    pub const COPILOT: &'static str = "copilot";
    pub const MCP_TESTER: &'static str = "mcp-tester";
}
```
Keep Display and FromStr impls so serialization stays compatible. Remove `all()` method if nothing uses it externally.

Replace `Model` enum (large list of variants) with a string newtype:
```
pub struct Model(pub String);
```
Keep Display and FromStr impls. The `all()` method can be removed (no longer an exhaustive list).

Remove `model_name_for_tool()` entirely — model strings are now passed verbatim to executors.

Update `StageInfo`:
- `tool: Option<Tool>` → `tool: Option<String>` (provider name, for display/logging)
- `model: Option<Model>` → `model: Option<String>` (raw model string)

## Why

The old enums forced a closed set of models that needed updating for every new model name. Making them open strings lets operators configure arbitrary model names in zbobr.toml. Removing `model_name_for_tool` removes the need for a mapping table that was always out of date.

## Important

This will cause compile errors in executor configs and dispatcher code that used the old enum — those are fixed in subsequent checklist items. Fix them as you encounter them rather than trying to keep old compatibility shims.
