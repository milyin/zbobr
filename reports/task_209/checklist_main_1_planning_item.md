# Config types refactor (zbobr-api/src/config.rs)

## What to change

Add two new structs:
- `ProviderDefinition`: fields `executor: Option<String>`, `parent: Option<String>`, `priority: i32` (default 10), `plan_mode: Option<bool>`, `access_key: Option<Secret>`. Derives Serialize, Deserialize, Clone, Debug.
- `ToolEntry`: fields `provider: String`, `model: String`. Derives Serialize, Deserialize, Clone, Debug.

Update `ZbobrDispatcherConfig`:
- Remove `tool: Tool` and `model: Model` fields
- Add `tool: String` (default global tool name, e.g. "smart")
- Add `providers: IndexMap<String, ProviderDefinition>`
- Add `tools: IndexMap<String, Vec<ToolEntry>>`
- Add `provider_exclusion_secs: u64` (default 300)

Update `RoleDefinition`:
- Remove `default_tool: Option<Tool>`, `default_model: Option<Model>`, `default_plan_mode: Option<bool>`
- Add `tool: Option<String>` (optional override tool name)

Update `StageDefinition`:
- Remove `tool: Option<Tool>`, `model: Option<Model>`, `plan_mode: Option<bool>`
- Add `tool: Option<String>` (optional override tool name)

Add method `resolve_tool_name(&self, stage_def: &StageDefinition, workflow: &WorkflowConfig) -> String`:
- Priority: stage.tool → role.tool → global dispatcher.tool
- Replaces the three old helpers: `tool_for_stage`, `model_for_stage`, `plan_mode_for_stage`

Add method `resolve_providers() -> HashMap<String, ResolvedProvider>` (or inline at build time):
- Flatten provider inheritance chains: for each provider, walk `parent` chain merging fields (child overrides parent)
- Detect cycles (error if parent chain loops)
- `ResolvedProvider` = `executor: String, priority: i32, plan_mode: bool, access_key: Option<Secret>`

Update `validate()`:
- For every tool entry, verify the referenced provider exists in providers map
- For every provider with a parent, verify parent exists
- Remove old model compatibility validation (`model_name_for_tool` check)

## Why

This is the central config layer that all other components read. Getting the types right here unblocks all downstream changes. The inheritance resolution at build/validate time keeps dispatch logic simple — it works with flat resolved providers only.

## Analog

Follow the same pattern as the existing `tool_for_stage` / `model_for_stage` resolution chain, but collapse it into a single `resolve_tool_name` that returns a String.
