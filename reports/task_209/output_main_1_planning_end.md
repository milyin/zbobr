The plan is ready for your review. Here's the summary:

## Proposed Approach

**New config structure** under `[dispatcher]`:
- `[dispatcher.providers]` — named executor configurations with `executor`, `parent` (inheritance), `priority`, `plan_mode`, `access_key`
- `[dispatcher.tools]` — named lists of `(provider, model)` pairs

**Breaking changes:**
- `RoleDefinition`: remove `default_tool/default_model/default_plan_mode`, add single `tool: String`
- `StageDefinition`: remove `tool/model/plan_mode`, add single `tool: String`  
- `Model` type: enum → arbitrary `String` newtype (passes verbatim to executor CLIs)
- `ZbobrDispatcherConfig`: replace `tool: Tool` + `model: Model` with `tool: String` (default tool name)

**New dispatch logic** in `ZbobrDispatcher`:
- Resolve tool name (stage → role → global)
- Filter temporarily excluded providers
- Select by priority (descending), round-robin within same tier
- On executor failure → `exclude_provider()` adds to timed exclusion map

**Init template** gets three example tools (`smart`, `planner`, `silly`) each with both copilot and claude entries to demonstrate the fallback pattern.

Key question before approving: should `providers`/`tools` live under `[dispatcher.*]` in TOML (co-located with dispatch config), or as separate top-level sections `[providers]`/`[tools]` as shown in the task description examples? The plan uses `[dispatcher.*]` for architectural cleanliness, but I can switch to top-level if you prefer matching the task examples exactly.