Remove all traces of the `DeleteCtxRec` variant from the tool registry and role configuration.

**What to change:**
- `zbobr-api/src/config_tools.rs`: Remove `DeleteCtxRec` variant from the `McpTool` enum, from `ALL_TOOLS` and `ALL_TOOL_NAMES` constants, from `as_str()` match arm, and from `FromStr` match arm.
- `zbobr/src/init.rs`: Remove `DeleteCtxRec` from the `use McpTool::{ ... }` import, and remove it from the `mcp` vectors of all four roles that currently include it: `planner`, `worker`, `test_planner`, `test_worker`.

**Why:** `DeleteCtxRec` is being removed as an allowed agent operation. Keeping it in the enum or role lists would cause compile errors or expose the tool unintentionally.