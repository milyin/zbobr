# Implementation Summary

## Changes Made

### 1. `zbobr-api/src/config.rs`
Added `priority: Option<i32>` field to `ToolEntry` struct with serde annotations `#[serde(default, skip_serializing_if = "Option::is_none")]`, matching the pattern used by `ProviderDefinition.priority`. Updated 6 test `ToolEntry` literals to include `priority: None`.

### 2. `zbobr-dispatcher/src/lib.rs`
- Updated `select_provider_excluding` to use `entry.priority.unwrap_or(rp.priority)` instead of `rp.priority` when grouping entries by priority tier.
- Added `priority: None` to the `tool_entry` test helper function.
- Added new test `select_provider_entry_priority_overrides_provider` that verifies: when two entries have providers both at priority 10, but one entry has `priority: Some(0)`, the override takes effect and the overridden entry is treated as lower priority.

### 3. `zbobr-dispatcher/tests/mcp_integration/env.rs`
Added `priority: None` to the `ToolEntry` literal.

### 4. `zbobr/src/init.rs`
- Removed `copilot_planner` provider from the example config (keeping `claude`, `copilot`, `claude_planner`).
- Renamed `smart` tool to `developer` and reordered entries: `claude` first (no priority), `copilot` second with `priority: Some(0)` as fallback.
- Simplified `planner` tool to only use `claude_planner` (removed `copilot_planner` reference).
- Updated all 4 role definitions that referenced `"smart"` to use `"developer"`.
- Added `inline_dispatcher_tables` function that converts provider sub-tables to inline tables and tool array-of-tables to inline arrays.
- Called `inline_dispatcher_tables` after `inline_stage_tables` in the config generation pipeline.

All tests pass.
