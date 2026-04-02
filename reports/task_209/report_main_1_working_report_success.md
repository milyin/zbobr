# Task 209: Separate Executor Settings with Fallbacks

## Summary

Implemented a layered provider/tool configuration system replacing the old static `tool/model/plan_mode` triple. All 6 checklist items completed in a single commit (`11af4c7`).

## Changes

### zbobr-api/src/config.rs
- Added `ProviderDefinition` (executor, parent, priority, plan_mode, access_key)
- Added `ResolvedProvider` (flattened after inheritance resolution)
- Added `ToolEntry` (provider + model pair)
- `ZbobrDispatcherConfig`: added `providers`, `tools`, `provider_exclusion_secs`; `tool` field is now `String`
- New methods: `resolve_tool_name()`, `resolve_providers()`, `resolve_single_provider()` (private)
- `validate()` now checks all provider/tool references
- Removed `tool_for_stage()`, `model_for_stage()`, `plan_mode_for_stage()`

### zbobr-api/src/task.rs
- `Tool` enum → newtype `pub struct Tool(pub String)` with constants `CLAUDE`, `COPILOT`, `MCP_TESTER`
- `Model` enum → newtype `pub struct Model(pub String)`
- `StageInfo.tool: Option<Tool>` → `Option<String>`
- `StageInfo.model: Option<Model>` → `Option<String>`
- Removed `model_name_for_tool()`

### zbobr-api/src/tool_executor.rs
- Added `model: &str` parameter to `ToolExecutor::execute`

### zbobr-api/src/context/stage_title.rs
- `MdStageTitle.tool: Option<Tool>` → `Option<String>`
- `MdStageTitle.model: Option<Model>` → `Option<String>`
- Updated `From<&StageInfo>`, `From<MdStageTitle>`, `FromStr`, and tests

### zbobr-executor-claude/src/config.rs
- Removed `default_model` field; `ZbobrExecutorClaudeConfig` is now empty (no TOML fields)

### zbobr-executor-claude/src/lib.rs
- Added `access_key: Option<Secret>` field; sets `ANTHROPIC_API_KEY` env var when present
- `execute()` now accepts `model: &str` and uses it as `--model` arg

### zbobr-executor-copilot/src/config.rs
- Removed `default_model` field; only `copilot_github_token` remains

### zbobr-executor-copilot/src/lib.rs
- `execute()` now accepts `model: &str` and uses it directly

### zbobr-executor-mcp-tester/src/lib.rs
- Added `_model: &str` parameter to satisfy trait

### zbobr-dispatcher/src/lib.rs
- `ZbobrDispatcher` gains `excluded_providers` and `round_robin_state` fields
- New `select_provider(tool_name)`: filters excluded providers, groups by priority, round-robins within top tier
- New `exclude_provider(provider_name)`: marks provider excluded for `provider_exclusion_secs`
- New `build_executor(provider, mcp_tester_override)`: dispatches to correct executor by `provider.executor`
- `ClaudeExecutor` gets `access_key` set from provider at build time

### zbobr-dispatcher/src/cli.rs
- Stage runner now calls `resolve_tool_name()` + `select_provider()` instead of old triple
- Stores provider name in `StageInfo.tool`, model string in `StageInfo.model`
- Provider exclusion on execution error
- `execute_tool` gains `model: &str` parameter

### zbobr/src/init.rs
- Template uses new provider/tool structure: providers (claude, copilot, claude_planner, copilot_planner) and tools (smart, planner, silly)
- Planner role uses `tool: Some("planner".to_string())` instead of `default_plan_mode`
- Removed ZbobrExecutorClaudeToml from template (no TOML fields)

## Test Results
All tests pass (`cargo test` — no failures).
